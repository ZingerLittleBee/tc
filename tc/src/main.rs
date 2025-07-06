use anyhow::Context;
use aya::maps::HashMap;
use aya::programs::{Xdp, XdpFlags};
use aya_log::EbpfLogger;
use clap::Parser;
use log::{debug, info, warn, LevelFilter};
use std::env;
use std::time::Duration;
use tc_common::TrafficStats;
use tokio::signal;

use crate::target_ip::{get_target_ip, TargetIp};

mod target_ip;
mod utils;

#[derive(Debug, Parser)]
struct Opt {
    #[clap(short, long, default_value = "eth0")]
    iface: String,
}

async fn display_traffic_stats(
    traffic_map: &HashMap<&aya::maps::MapData, u32, TrafficStats>,
    target_ip: &Vec<TargetIp>,
) -> Result<(), anyhow::Error> {
    for ip in target_ip {
        if let Ok(stats) = traffic_map.get(&ip.0, 0) {
            let ip_addr = ip.to_string();
            info!("\n=== 流量统计 for {} ===", ip_addr);
            info!("入站流量:");
            info!("  数据包: {} 个", stats.inbound_packets);
            info!(
                "  字节数: {} bytes ({:.2} KB)",
                stats.inbound_bytes,
                stats.inbound_bytes as f64 / 1024.0
            );
            info!("出站流量:");
            info!("  数据包: {} 个", stats.outbound_packets);
            info!(
                "  字节数: {} bytes ({:.2} KB)",
                stats.outbound_bytes,
                stats.outbound_bytes as f64 / 1024.0
            );
            info!("总计:");
            info!("  数据包: {} 个", stats.total_packets());
            info!(
                "  字节数: {} bytes ({:.2} KB)",
                stats.total_bytes(),
                stats.total_bytes() as f64 / 1024.0
            );
            info!("================================\n");
        } else {
            info!("没有找到 {} 的流量统计数据", ip.to_string());
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

    let mut xdp_target_ip_map: HashMap<_, u32, u8> =
        HashMap::try_from(bpf.map_mut("TARGET_IP").unwrap())?;

    for ip in target_ip.clone() {
        info!(
            "insert xdp_target_ip_map: {:?}, to: {:?}",
            ip.0,
            ip.to_string()
        );
        xdp_target_ip_map.insert(&ip.0, &1u8, 0)?;
    }

    // 获取流量统计Map
    let traffic_map: HashMap<_, u32, TrafficStats> =
        HashMap::try_from(bpf.map("TRAFFIC_STATS").unwrap())?;

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

    // 定期显示统计信息
    loop {
        tokio::select! {
            _ = async {
                tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_secs(5))).await
            } => {
                if let Err(e) = display_traffic_stats(&traffic_map, &target_ip).await {
                    warn!("显示统计信息时出错: {}", e);
                }
            }
            _ = signal::ctrl_c() => {
                info!("收到 Ctrl-C 信号，正在退出...");
                // 最后显示一次统计信息
                let _ = display_traffic_stats(&traffic_map, &target_ip).await;
                break;
            }
        }
    }

    Ok(())
}
