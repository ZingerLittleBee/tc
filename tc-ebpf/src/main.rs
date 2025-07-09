#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::HashMap,
    programs::XdpContext,
};
use aya_log_ebpf::info;

use core::mem;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

use tc_common::{
    EnhancedTrafficStats, FlowKey, PortStats, ProtocolStats, DIRECTION_INBOUND, DIRECTION_OUTBOUND,
    PROTOCOL_TCP, PROTOCOL_UDP,
};

// === eBPF Maps 定义 ===

// 目标 IP 配置
#[map]
static TARGET_IP: HashMap<u32, u8> = HashMap::with_max_entries(1024, 0);

// 多维度流量统计：IP + Port + Protocol + Direction
#[map]
static FLOW_STATS: HashMap<FlowKey, EnhancedTrafficStats> = HashMap::with_max_entries(8192, 0);

// 每个 IP 的协议统计
#[map]
static IP_PROTOCOL_STATS: HashMap<u32, ProtocolStats> = HashMap::with_max_entries(1024, 0);

// 热门端口统计
#[map]
static PORT_STATS: HashMap<u16, PortStats> = HashMap::with_max_entries(1024, 0);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[xdp]
pub fn xdp_firewall(ctx: XdpContext) -> u32 {
    match try_xdp_firewall(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

fn try_xdp_firewall(ctx: XdpContext) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = ptr_at(&ctx, 0)?;
    match unsafe { (*ethhdr).ether_type } {
        EtherType::Ipv4 => {}
        _ => return Ok(xdp_action::XDP_PASS),
    }

    let ipv4hdr: *const Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
    let source_addr = u32::from_be_bytes(unsafe { (*ipv4hdr).src_addr });
    let dest_addr = u32::from_be_bytes(unsafe { (*ipv4hdr).dst_addr });
    let packet_len = u16::from_be_bytes(unsafe { (*ipv4hdr).tot_len }) as u64;
    let protocol = unsafe { (*ipv4hdr).proto };

    // 解析端口信息
    let (source_port, dest_port) = match protocol {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            (
                u16::from_be(unsafe { (*tcphdr).source }),
                u16::from_be(unsafe { (*tcphdr).dest }),
            )
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            (
                u16::from_be_bytes(unsafe { (*udphdr).source }),
                u16::from_be_bytes(unsafe { (*udphdr).dest }),
            )
        }
        _ => return Ok(xdp_action::XDP_PASS), // 只处理 TCP/UDP
    };

    // 获取当前时间戳
    let current_time = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    let protocol_type = match protocol {
        IpProto::Tcp => PROTOCOL_TCP,
        IpProto::Udp => PROTOCOL_UDP,
        _ => return Ok(xdp_action::XDP_PASS),
    };

    // 检查源 IP 是否为监控目标 (入站流量)
    if unsafe { TARGET_IP.get(&source_addr) }.is_some() {
        info!(
            &ctx,
            "INBOUND - IP: {:i}, PORT: {}, PROTO: {}, SIZE: {} bytes",
            source_addr,
            source_port,
            protocol_type,
            packet_len
        );

        // 更新多维度流量统计
        update_flow_stats(
            source_addr,
            source_port,
            protocol_type,
            DIRECTION_INBOUND,
            packet_len,
            current_time,
        );

        // 更新协议统计
        update_protocol_stats(source_addr, protocol_type, packet_len, 1);

        // 更新端口统计
        update_port_stats(source_port, protocol_type, packet_len, current_time);
    }

    // 检查目标 IP 是否为监控目标 (出站流量)
    if unsafe { TARGET_IP.get(&dest_addr) }.is_some() {
        info!(
            &ctx,
            "OUTBOUND - IP: {:i}, PORT: {}, PROTO: {}, SIZE: {} bytes",
            dest_addr,
            dest_port,
            protocol_type,
            packet_len
        );

        // 更新多维度流量统计
        update_flow_stats(
            dest_addr,
            dest_port,
            protocol_type,
            DIRECTION_OUTBOUND,
            packet_len,
            current_time,
        );

        // 更新协议统计
        update_protocol_stats(dest_addr, protocol_type, packet_len, 1);

        // 更新端口统计
        update_port_stats(dest_port, protocol_type, packet_len, current_time);
    }

    Ok(xdp_action::XDP_PASS)
}

#[inline(always)]
fn update_flow_stats(ip: u32, port: u16, protocol: u8, direction: u8, bytes: u64, timestamp: u64) {
    let key = FlowKey {
        ip,
        port,
        protocol,
        direction,
    };

    // 安全的方式：先尝试获取，如果不存在则创建新的
    if let Some(existing_stats) = unsafe { FLOW_STATS.get(&key) } {
        // 存在现有统计数据，更新它
        let mut stats = *existing_stats;
        
        // 根据方向更新统计
        if direction == DIRECTION_INBOUND {
            stats.inbound_packets += 1;
            stats.inbound_bytes += bytes;
        } else {
            stats.outbound_packets += 1;
            stats.outbound_bytes += bytes;
        }

        stats.last_seen = timestamp;
        stats.connection_count += 1;

        let _ = unsafe { FLOW_STATS.insert(&key, &stats, 0) };
    } else {
        // 创建新的统计数据
        let stats = EnhancedTrafficStats {
            inbound_packets: if direction == DIRECTION_INBOUND { 1 } else { 0 },
            inbound_bytes: if direction == DIRECTION_INBOUND { bytes } else { 0 },
            outbound_packets: if direction != DIRECTION_INBOUND { 1 } else { 0 },
            outbound_bytes: if direction != DIRECTION_INBOUND { bytes } else { 0 },
            protocol,
            last_seen: timestamp,
            connection_count: 1,
            _padding: [0; 3],
        };

        let _ = unsafe { FLOW_STATS.insert(&key, &stats, 0) };
    }
}

#[inline(always)]
fn update_protocol_stats(ip: u32, protocol: u8, bytes: u64, packets: u64) {
    if let Some(existing_stats) = unsafe { IP_PROTOCOL_STATS.get(&ip) } {
        let mut stats = *existing_stats;
        
        match protocol {
            PROTOCOL_TCP => {
                stats.tcp_flows += 1;
                stats.tcp_bytes += bytes;
                stats.tcp_packets += packets;
            }
            PROTOCOL_UDP => {
                stats.udp_flows += 1;
                stats.udp_bytes += bytes;
                stats.udp_packets += packets;
            }
            _ => return,
        }

        let _ = unsafe { IP_PROTOCOL_STATS.insert(&ip, &stats, 0) };
    } else {
        let stats = match protocol {
            PROTOCOL_TCP => ProtocolStats {
                tcp_flows: 1,
                udp_flows: 0,
                tcp_bytes: bytes,
                udp_bytes: 0,
                tcp_packets: packets,
                udp_packets: 0,
            },
            PROTOCOL_UDP => ProtocolStats {
                tcp_flows: 0,
                udp_flows: 1,
                tcp_bytes: 0,
                udp_bytes: bytes,
                tcp_packets: 0,
                udp_packets: packets,
            },
            _ => return,
        };

        let _ = unsafe { IP_PROTOCOL_STATS.insert(&ip, &stats, 0) };
    }
}

#[inline(always)]
fn update_port_stats(port: u16, protocol: u8, bytes: u64, timestamp: u64) {
    if let Some(existing_stats) = unsafe { PORT_STATS.get(&port) } {
        let mut stats = *existing_stats;
        
        stats.total_bytes += bytes;
        stats.total_packets += 1;
        stats.active_connections += 1;
        stats.last_active = timestamp;

        let _ = unsafe { PORT_STATS.insert(&port, &stats, 0) };
    } else {
        let stats = PortStats {
            port,
            protocol,
            _padding: 0,
            total_bytes: bytes,
            total_packets: 1,
            active_connections: 1,
            last_active: timestamp,
        };

        let _ = unsafe { PORT_STATS.insert(&port, &stats, 0) };
    }
}
