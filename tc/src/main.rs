use anyhow::Context;
use aya::maps::HashMap;
use aya::programs::{Xdp, XdpFlags};
use aya_log::EbpfLogger;
use clap::Parser;
use log::{debug, info, warn, LevelFilter};
use std::env;
use std::time::Duration;
use tc_common::{EnhancedTrafficStats, FlowKey, PortStats, ProtocolStats, TrafficStats};
use tokio::signal;

use crate::analytics::TrafficAnalyzer;
use crate::storage::TrafficStorage;
use crate::web_api::{start_web_server, AppState};

use crate::target_ip::{get_target_ip, TargetIp};

mod analytics;
mod serializable_types;
mod storage;
mod target_ip;
mod utils;
mod web_api;

#[derive(Debug, Parser)]
struct Opt {
    #[clap(short, long, default_value = "eth0")]
    iface: String,

    #[clap(short, long, default_value = "8080")]
    port: u16,

    #[clap(long, default_value = "false")]
    disable_web: bool,
}

async fn collect_and_store_data(
    storage: &TrafficStorage,
    flow_stats: &HashMap<&aya::maps::MapData, FlowKey, EnhancedTrafficStats>,
    protocol_stats: &HashMap<&aya::maps::MapData, u32, ProtocolStats>,
    port_stats: &HashMap<&aya::maps::MapData, u16, PortStats>,
) -> Result<(), anyhow::Error> {
    // 收集流量数据
    let mut flows = std::collections::HashMap::new();
    for item in flow_stats.iter() {
        if let Ok((key, stats)) = item {
            flows.insert(key, stats);
        }
    }

    // 收集协议统计数据
    let mut protocols = std::collections::HashMap::new();
    for item in protocol_stats.iter() {
        if let Ok((ip, stats)) = item {
            protocols.insert(ip, stats);
        }
    }

    // 收集端口统计数据
    let mut ports = std::collections::HashMap::new();
    for item in port_stats.iter() {
        if let Ok((port, stats)) = item {
            ports.insert(port, stats);
        }
    }

    // 存储到RocksDB
    storage.store_traffic_snapshot(&flows, &protocols, &ports)?;

    debug!(
        "已存储 {} 个流量记录, {} 个协议记录, {} 个端口记录",
        flows.len(),
        protocols.len(),
        ports.len()
    );

    Ok(())
}

async fn display_enhanced_traffic_stats(
    analyzer: &mut TrafficAnalyzer,
    flow_stats: &HashMap<&aya::maps::MapData, FlowKey, EnhancedTrafficStats>,
    protocol_stats: &HashMap<&aya::maps::MapData, u32, ProtocolStats>,
    port_stats: &HashMap<&aya::maps::MapData, u16, PortStats>,
) -> Result<(), anyhow::Error> {
    match analyzer.analyze_ebpf_data(flow_stats, protocol_stats, port_stats) {
        Ok(dashboard_data) => {
            let metrics = &dashboard_data.realtime_metrics;

            info!("\n=== 实时监控统计 ===");
            info!(
                "总带宽: {:.2} KB/s",
                metrics.total_bandwidth_bps as f64 / 1024.0
            );
            info!("包速率: {} pps", metrics.total_packet_rate_pps);
            info!("活跃流: {} 个", metrics.active_flows);
            info!("活跃IP: {} 个", metrics.active_ips);
            info!("TCP连接: {} 个", metrics.tcp_connections);
            info!("UDP连接: {} 个", metrics.udp_connections);

            info!("\n=== Top 5 活跃IP ===");
            for (i, ip_summary) in dashboard_data.top_ips.iter().take(5).enumerate() {
                info!(
                    "{}. {} - 总流量: {:.2} KB (入: {:.2} KB, 出: {:.2} KB)",
                    i + 1,
                    ip_summary.ip,
                    (ip_summary.inbound_bytes + ip_summary.outbound_bytes) as f64 / 1024.0,
                    ip_summary.inbound_bytes as f64 / 1024.0,
                    ip_summary.outbound_bytes as f64 / 1024.0
                );
            }

            info!("\n=== Top 5 活跃端口 ===");
            for (i, port_summary) in dashboard_data.top_ports.iter().take(5).enumerate() {
                let service_info = port_summary
                    .service_name
                    .as_ref()
                    .map(|s| format!(" ({})", s))
                    .unwrap_or_default();
                info!(
                    "{}. 端口 {}{} - {} - {:.2} KB",
                    i + 1,
                    port_summary.port,
                    service_info,
                    port_summary.protocol,
                    port_summary.total_bytes as f64 / 1024.0
                );
            }

            info!("\n=== 协议分布 ===");
            let proto = &dashboard_data.protocol_breakdown;
            info!(
                "TCP: {:.1}% ({:.2} KB)",
                proto.tcp_percentage,
                proto.tcp_bytes as f64 / 1024.0
            );
            info!(
                "UDP: {:.1}% ({:.2} KB)",
                proto.udp_percentage,
                proto.udp_bytes as f64 / 1024.0
            );
            info!("================================\n");
        }
        Err(e) => {
            warn!("分析流量数据时出错: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    let opt = Opt::parse();

    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {ret}");
    }

    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Ebpf::load_file` instead.
    let mut bpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/tc")))?;
    if let Err(e) = EbpfLogger::init(&mut bpf) {
        // This can happen if you remove all log statements from your eBPF program.
        warn!("failed to initialize eBPF logger: {e}");
    }

    let program: &mut Xdp = bpf.program_mut("xdp_firewall").unwrap().try_into()?;
    program.load()?;
    program.attach(&opt.iface, XdpFlags::default())
        .context("failed to attach the XDP program with default flags - try changing XdpFlags::default() to XdpFlags::SKB_MODE")?;

    let target_ip = get_target_ip()?;

    // 配置目标IP到eBPF
    let mut xdp_target_ip_map: HashMap<_, u32, u8> =
        HashMap::try_from(bpf.map_mut("TARGET_IP").unwrap())?;

    for ip in target_ip.clone() {
        info!("添加目标IP到监控列表: {} ({})", ip.to_string(), ip.0);
        xdp_target_ip_map.insert(&ip.0, &1u8, 0)?;
    }

    // 获取增强的eBPF Maps
    let flow_stats_map: HashMap<_, FlowKey, EnhancedTrafficStats> =
        HashMap::try_from(bpf.map("FLOW_STATS").unwrap())?;
    let protocol_stats_map: HashMap<_, u32, ProtocolStats> =
        HashMap::try_from(bpf.map("IP_PROTOCOL_STATS").unwrap())?;
    let port_stats_map: HashMap<_, u16, PortStats> =
        HashMap::try_from(bpf.map("PORT_STATS").unwrap())?;

    // 保留原有的简单统计map（用于向后兼容）
    // let traffic_map: HashMap<_, u32, TrafficStats> =
    //     HashMap::try_from(bpf.map("TRAFFIC_STATS").unwrap())
    //         .ok()
    //         .unwrap_or_else(|| {
    //             warn!("TRAFFIC_STATS map 不存在，仅使用增强统计功能");
    //             // 创建一个空的map作为占位符，但实际不会使用
    //             HashMap::try_from(bpf.map("TARGET_IP").unwrap()).unwrap()
    //         });

    // 初始化存储和分析器
    let storage = TrafficStorage::new("./traffic_data").context("初始化RocksDB存储失败")?;
    let mut analyzer = TrafficAnalyzer::new();

    // 初始化 Web API 状态
    let api_state = AppState::new(storage);

    info!("已初始化数据存储和分析器");

    info!("XDP程序已加载并附加到 {} 接口", opt.iface);
    info!(
        "开始监控 [{}] 的流量...",
        target_ip
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );
    info!("按 Ctrl-C 退出");

    // 启动 Web API 服务器（如果启用）
    if !opt.disable_web {
        let api_state_clone = api_state.clone();
        let web_port = opt.port;
        tokio::spawn(async move {
            if let Err(e) = start_web_server(api_state_clone, web_port).await {
                eprintln!("Web API 服务器启动失败: {}", e);
            }
        });

        // 给 Web 服务器一点时间启动
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // 定期显示统计信息并存储数据
    loop {
        tokio::select! {
            _ = async {
                tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_secs(5))).await
            } => {
                // 显示增强的统计信息并更新 API 数据
                if let Err(e) = display_enhanced_traffic_stats(
                    &mut analyzer,
                    &flow_stats_map,
                    &protocol_stats_map,
                    &port_stats_map
                ).await {
                    warn!("显示统计信息时出错: {}", e);
                }

                // 分析数据并更新到 API 状态
                if let Ok(dashboard_data) = analyzer.analyze_ebpf_data(
                    &flow_stats_map,
                    &protocol_stats_map,
                    &port_stats_map
                ) {
                    api_state.update_dashboard_data(dashboard_data).await;
                }

                // 收集数据并存储到RocksDB
                if let Err(e) = collect_and_store_data(
                    &api_state.storage,
                    &flow_stats_map,
                    &protocol_stats_map,
                    &port_stats_map
                ).await {
                    warn!("存储数据时出错: {}", e);
                }
            }
            _ = signal::ctrl_c() => {
                info!("收到 Ctrl-C 信号，正在退出...");

                // 最后一次数据收集和显示
                let _ = display_enhanced_traffic_stats(
                    &mut analyzer,
                    &flow_stats_map,
                    &protocol_stats_map,
                    &port_stats_map
                ).await;

                let _ = collect_and_store_data(
                    &api_state.storage,
                    &flow_stats_map,
                    &protocol_stats_map,
                    &port_stats_map
                ).await;

                info!("数据已保存，程序退出");
                break;
            }
        }
    }

    Ok(())
}
