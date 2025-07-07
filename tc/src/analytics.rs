use anyhow::Result;
use aya::maps::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use tc_common::{
    EnhancedTrafficStats, FlowKey, PortStats, ProtocolStats, PROTOCOL_TCP, PROTOCOL_UDP,
};

// 前端展示数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DashboardData {
    pub realtime_metrics: RealtimeMetrics,
    pub top_ips: Vec<IpTrafficSummary>,
    pub top_ports: Vec<PortTrafficSummary>,
    pub protocol_breakdown: ProtocolBreakdown,
    pub timeline_data: Vec<TimelinePoint>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RealtimeMetrics {
    pub total_bandwidth_bps: u64,    // 总带宽 (bytes/second)
    pub total_packet_rate_pps: u64,  // 包速率 (packets/second)
    pub active_flows: u32,           // 活跃流数量
    pub active_ips: u32,             // 活跃 IP 数量
    pub tcp_connections: u32,        // TCP 连接数
    pub udp_connections: u32,        // UDP 连接数
    pub last_updated: DateTime<Utc>, // 最后更新时间
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IpTrafficSummary {
    pub ip: String,                   // IP 地址
    pub inbound_bytes: u64,           // 入站字节数
    pub outbound_bytes: u64,          // 出站字节数
    pub inbound_packets: u64,         // 入站包数
    pub outbound_packets: u64,        // 出站包数
    pub total_flows: u32,             // 总流数量
    pub top_ports: Vec<u16>,          // 主要使用的端口
    pub protocols: ProtocolBreakdown, // 协议分布
    pub last_active: DateTime<Utc>,   // 最后活跃时间
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortTrafficSummary {
    pub port: u16,                    // 端口号
    pub service_name: Option<String>, // 服务名称 (HTTP, HTTPS, SSH 等)
    pub protocol: String,             // 主要协议 (TCP/UDP)
    pub total_bytes: u64,             // 总字节数
    pub total_packets: u64,           // 总包数
    pub active_connections: u32,      // 活跃连接数
    pub associated_ips: Vec<String>,  // 相关 IP 地址
    pub last_active: DateTime<Utc>,   // 最后活跃时间
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolBreakdown {
    pub tcp_bytes: u64,      // TCP 字节数
    pub tcp_packets: u64,    // TCP 包数
    pub tcp_flows: u32,      // TCP 流数
    pub udp_bytes: u64,      // UDP 字节数
    pub udp_packets: u64,    // UDP 包数
    pub udp_flows: u32,      // UDP 流数
    pub tcp_percentage: f64, // TCP 流量占比
    pub udp_percentage: f64, // UDP 流量占比
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimelinePoint {
    pub timestamp: DateTime<Utc>, // 时间戳
    pub total_bytes: u64,         // 总字节数
    pub total_packets: u64,       // 总包数
    pub tcp_bytes: u64,           // TCP 字节数
    pub udp_bytes: u64,           // UDP 字节数
    pub active_flows: u32,        // 活跃流数量
}

// 数据分析器主要结构
pub struct TrafficAnalyzer {
    last_snapshot_time: DateTime<Utc>,
    previous_totals: BTreeMap<String, u64>, // 用于计算速率
}

impl TrafficAnalyzer {
    pub fn new() -> Self {
        Self {
            last_snapshot_time: Utc::now(),
            previous_totals: BTreeMap::new(),
        }
    }

    // 从 eBPF maps 分析数据并生成仪表板数据
    pub fn analyze_ebpf_data(
        &mut self,
        flow_stats: &HashMap<&aya::maps::MapData, FlowKey, EnhancedTrafficStats>,
        protocol_stats: &HashMap<&aya::maps::MapData, u32, ProtocolStats>,
        port_stats: &HashMap<&aya::maps::MapData, u16, PortStats>,
    ) -> Result<DashboardData> {
        let current_time = Utc::now();
        let time_diff_secs = (current_time - self.last_snapshot_time).num_seconds() as u64;
        let time_diff_secs = if time_diff_secs == 0 {
            1
        } else {
            time_diff_secs
        }; // 避免除零

        // 收集所有流量数据
        let mut flow_data = std::collections::HashMap::new();
        for item in flow_stats.iter() {
            if let Ok((key, stats)) = item {
                flow_data.insert(key, stats);
            }
        }

        // 收集协议统计数据
        let mut protocol_data = std::collections::HashMap::new();
        for item in protocol_stats.iter() {
            if let Ok((ip, stats)) = item {
                protocol_data.insert(ip, stats);
            }
        }

        // 收集端口统计数据
        let mut port_data = std::collections::HashMap::new();
        for item in port_stats.iter() {
            if let Ok((port, stats)) = item {
                port_data.insert(port, stats);
            }
        }

        // 计算实时指标
        let realtime_metrics = self.calculate_realtime_metrics(
            &flow_data,
            &protocol_data,
            time_diff_secs,
            current_time,
        );

        // 计算 Top IPs
        let top_ips = self.calculate_top_ips(&flow_data, &protocol_data);

        // 计算 Top Ports
        let top_ports = self.calculate_top_ports(&port_data, &flow_data);

        // 计算协议分布
        let protocol_breakdown = self.calculate_protocol_breakdown(&protocol_data);

        // 生成时间线数据点
        let timeline_point = self.generate_timeline_point(&flow_data, current_time);

        self.last_snapshot_time = current_time;

        Ok(DashboardData {
            realtime_metrics,
            top_ips,
            top_ports,
            protocol_breakdown,
            timeline_data: vec![timeline_point], // 单个时间点，在实际应用中应维护一个时间序列
        })
    }

    fn calculate_realtime_metrics(
        &mut self,
        flows: &std::collections::HashMap<FlowKey, EnhancedTrafficStats>,
        protocols: &std::collections::HashMap<u32, ProtocolStats>,
        time_diff_secs: u64,
        current_time: DateTime<Utc>,
    ) -> RealtimeMetrics {
        let mut total_bytes = 0u64;
        let mut total_packets = 0u64;
        let mut active_flows = 0u32;
        let mut tcp_connections = 0u32;
        let mut udp_connections = 0u32;
        let mut active_ips = std::collections::HashSet::new();

        // 聚合流量数据
        for (flow_key, stats) in flows {
            total_bytes += stats.total_bytes();
            total_packets += stats.total_packets();
            active_flows += 1;
            active_ips.insert(flow_key.ip);

            match flow_key.protocol {
                PROTOCOL_TCP => tcp_connections += stats.connection_count,
                PROTOCOL_UDP => udp_connections += stats.connection_count,
                _ => {}
            }
        }

        // 计算带宽（字节/秒）
        let previous_bytes = self
            .previous_totals
            .get("total_bytes")
            .copied()
            .unwrap_or(0);
        let bandwidth_bps = if total_bytes > previous_bytes {
            (total_bytes - previous_bytes) / time_diff_secs
        } else {
            0
        };

        // 计算包速率（包/秒）
        let previous_packets = self
            .previous_totals
            .get("total_packets")
            .copied()
            .unwrap_or(0);
        let packet_rate_pps = if total_packets > previous_packets {
            (total_packets - previous_packets) / time_diff_secs
        } else {
            0
        };

        // 更新之前的总计
        self.previous_totals
            .insert("total_bytes".to_string(), total_bytes);
        self.previous_totals
            .insert("total_packets".to_string(), total_packets);

        RealtimeMetrics {
            total_bandwidth_bps: bandwidth_bps,
            total_packet_rate_pps: packet_rate_pps,
            active_flows,
            active_ips: active_ips.len() as u32,
            tcp_connections,
            udp_connections,
            last_updated: current_time,
        }
    }

    fn calculate_top_ips(
        &self,
        flows: &std::collections::HashMap<FlowKey, EnhancedTrafficStats>,
        protocols: &std::collections::HashMap<u32, ProtocolStats>,
    ) -> Vec<IpTrafficSummary> {
        let mut ip_aggregates: std::collections::HashMap<u32, IpTrafficSummary> =
            std::collections::HashMap::new();

        // 聚合每个 IP 的流量数据
        for (flow_key, stats) in flows {
            let entry = ip_aggregates
                .entry(flow_key.ip)
                .or_insert_with(|| IpTrafficSummary {
                    ip: Ipv4Addr::from(flow_key.ip).to_string(),
                    inbound_bytes: 0,
                    outbound_bytes: 0,
                    inbound_packets: 0,
                    outbound_packets: 0,
                    total_flows: 0,
                    top_ports: Vec::new(),
                    protocols: ProtocolBreakdown {
                        tcp_bytes: 0,
                        tcp_packets: 0,
                        tcp_flows: 0,
                        udp_bytes: 0,
                        udp_packets: 0,
                        udp_flows: 0,
                        tcp_percentage: 0.0,
                        udp_percentage: 0.0,
                    },
                    last_active: DateTime::from_timestamp(
                        stats.last_seen as i64 / 1_000_000_000,
                        0,
                    )
                    .unwrap_or(Utc::now()),
                });

            entry.inbound_bytes += stats.inbound_bytes;
            entry.outbound_bytes += stats.outbound_bytes;
            entry.inbound_packets += stats.inbound_packets;
            entry.outbound_packets += stats.outbound_packets;
            entry.total_flows += 1;

            // 更新协议统计
            match flow_key.protocol {
                PROTOCOL_TCP => {
                    entry.protocols.tcp_bytes += stats.total_bytes();
                    entry.protocols.tcp_packets += stats.total_packets();
                    entry.protocols.tcp_flows += 1;
                }
                PROTOCOL_UDP => {
                    entry.protocols.udp_bytes += stats.total_bytes();
                    entry.protocols.udp_packets += stats.total_packets();
                    entry.protocols.udp_flows += 1;
                }
                _ => {}
            }

            // 更新最后活跃时间
            let last_seen = DateTime::from_timestamp(stats.last_seen as i64 / 1_000_000_000, 0)
                .unwrap_or(Utc::now());
            if last_seen > entry.last_active {
                entry.last_active = last_seen;
            }
        }

        // 计算协议百分比并收集端口信息
        for (ip, summary) in ip_aggregates.iter_mut() {
            let total_bytes = summary.protocols.tcp_bytes + summary.protocols.udp_bytes;
            if total_bytes > 0 {
                summary.protocols.tcp_percentage =
                    (summary.protocols.tcp_bytes as f64 / total_bytes as f64) * 100.0;
                summary.protocols.udp_percentage =
                    (summary.protocols.udp_bytes as f64 / total_bytes as f64) * 100.0;
            }

            // 收集此 IP 使用的端口
            let mut ports: std::collections::HashMap<u16, u64> = std::collections::HashMap::new();
            for (flow_key, stats) in flows {
                if flow_key.ip == *ip {
                    *ports.entry(flow_key.port).or_insert(0) += stats.total_bytes();
                }
            }

            // 按流量排序，取前 5 个端口
            let mut sorted_ports: Vec<_> = ports.into_iter().collect();
            sorted_ports.sort_by(|a, b| b.1.cmp(&a.1));
            summary.top_ports = sorted_ports
                .into_iter()
                .take(5)
                .map(|(port, _)| port)
                .collect();
        }

        // 按总流量排序
        let mut sorted_ips: Vec<_> = ip_aggregates.into_values().collect();
        sorted_ips.sort_by(|a, b| {
            (b.inbound_bytes + b.outbound_bytes).cmp(&(a.inbound_bytes + a.outbound_bytes))
        });

        sorted_ips.into_iter().take(10).collect() // 返回前 10 个
    }

    fn calculate_top_ports(
        &self,
        ports: &std::collections::HashMap<u16, PortStats>,
        flows: &std::collections::HashMap<FlowKey, EnhancedTrafficStats>,
    ) -> Vec<PortTrafficSummary> {
        let mut port_summaries = Vec::new();

        for (port, stats) in ports {
            // 收集与此端口相关的 IP 地址
            let mut associated_ips = std::collections::HashSet::new();
            for (flow_key, _) in flows {
                if flow_key.port == *port {
                    associated_ips.insert(Ipv4Addr::from(flow_key.ip).to_string());
                }
            }

            let summary = PortTrafficSummary {
                port: *port,
                service_name: get_service_name(*port),
                protocol: get_protocol_name(stats.protocol),
                total_bytes: stats.total_bytes,
                total_packets: stats.total_packets,
                active_connections: stats.active_connections,
                associated_ips: associated_ips.into_iter().collect(),
                last_active: DateTime::from_timestamp(stats.last_active as i64 / 1_000_000_000, 0)
                    .unwrap_or(Utc::now()),
            };

            port_summaries.push(summary);
        }

        // 按总字节数排序
        port_summaries.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        port_summaries.into_iter().take(10).collect() // 返回前 10 个
    }

    fn calculate_protocol_breakdown(
        &self,
        protocols: &std::collections::HashMap<u32, ProtocolStats>,
    ) -> ProtocolBreakdown {
        let mut tcp_bytes = 0u64;
        let mut tcp_packets = 0u64;
        let mut tcp_flows = 0u32;
        let mut udp_bytes = 0u64;
        let mut udp_packets = 0u64;
        let mut udp_flows = 0u32;

        for (_, stats) in protocols {
            tcp_bytes += stats.tcp_bytes;
            tcp_packets += stats.tcp_packets;
            tcp_flows += stats.tcp_flows;
            udp_bytes += stats.udp_bytes;
            udp_packets += stats.udp_packets;
            udp_flows += stats.udp_flows;
        }

        let total_bytes = tcp_bytes + udp_bytes;
        let tcp_percentage = if total_bytes > 0 {
            (tcp_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };
        let udp_percentage = if total_bytes > 0 {
            (udp_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        ProtocolBreakdown {
            tcp_bytes,
            tcp_packets,
            tcp_flows,
            udp_bytes,
            udp_packets,
            udp_flows,
            tcp_percentage,
            udp_percentage,
        }
    }

    fn generate_timeline_point(
        &self,
        flows: &std::collections::HashMap<FlowKey, EnhancedTrafficStats>,
        timestamp: DateTime<Utc>,
    ) -> TimelinePoint {
        let mut total_bytes = 0u64;
        let mut total_packets = 0u64;
        let mut tcp_bytes = 0u64;
        let mut udp_bytes = 0u64;
        let active_flows = flows.len() as u32;

        for (flow_key, stats) in flows {
            total_bytes += stats.total_bytes();
            total_packets += stats.total_packets();

            match flow_key.protocol {
                PROTOCOL_TCP => tcp_bytes += stats.total_bytes(),
                PROTOCOL_UDP => udp_bytes += stats.total_bytes(),
                _ => {}
            }
        }

        TimelinePoint {
            timestamp,
            total_bytes,
            total_packets,
            tcp_bytes,
            udp_bytes,
            active_flows,
        }
    }
}

// 辅助函数：根据端口号获取服务名称
fn get_service_name(port: u16) -> Option<String> {
    match port {
        22 => Some("SSH".to_string()),
        53 => Some("DNS".to_string()),
        80 => Some("HTTP".to_string()),
        443 => Some("HTTPS".to_string()),
        3306 => Some("MySQL".to_string()),
        5432 => Some("PostgreSQL".to_string()),
        6379 => Some("Redis".to_string()),
        3000 => Some("Development Server".to_string()),
        8080 => Some("HTTP Alt".to_string()),
        8443 => Some("HTTPS Alt".to_string()),
        _ => None,
    }
}

// 辅助函数：根据协议号获取协议名称
fn get_protocol_name(protocol: u8) -> String {
    match protocol {
        PROTOCOL_TCP => "TCP".to_string(),
        PROTOCOL_UDP => "UDP".to_string(),
        _ => format!("Protocol {}", protocol),
    }
}
