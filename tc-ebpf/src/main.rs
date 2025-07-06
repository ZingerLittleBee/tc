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

// 流量统计结构
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrafficStats {
    pub inbound_packets: u64,
    pub inbound_bytes: u64,
    pub outbound_packets: u64,
    pub outbound_bytes: u64,
}

// 定义流量统计Map
#[map]
static TRAFFIC_STATS: HashMap<u32, TrafficStats> = HashMap::with_max_entries(1024, 0);

#[map]
static TARGET_IP: HashMap<u32, u8> = HashMap::with_max_entries(1024, 0);

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

#[inline(always)] // (1)
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
    let ethhdr: *const EthHdr = ptr_at(&ctx, 0)?; // (2)
    match unsafe { (*ethhdr).ether_type } {
        EtherType::Ipv4 => {}
        _ => return Ok(xdp_action::XDP_PASS),
    }

    let ipv4hdr: *const Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
    let source_addr = u32::from_be_bytes(unsafe { (*ipv4hdr).src_addr });
    let dest_addr = u32::from_be_bytes(unsafe { (*ipv4hdr).dst_addr });

    // 计算数据包大小
    let packet_len = u16::from_be_bytes(unsafe { (*ipv4hdr).tot_len }) as u64;

    let source_port = match unsafe { (*ipv4hdr).proto } {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be(unsafe { (*tcphdr).source })
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be_bytes(unsafe { (*udphdr).source })
        }
        _ => return Err(()),
    };

    // 统计入站流量 (源IP是目标IP)
    if unsafe { TARGET_IP.get(&source_addr) }.is_some() {
        info!(
            &ctx,
            "INBOUND - SRC IP: {:i}, SRC PORT: {}, SIZE: {} bytes",
            source_addr,
            source_port,
            packet_len
        );

        // 更新入站流量统计
        let mut stats = unsafe { TRAFFIC_STATS.get(&source_addr) }
            .copied()
            .unwrap_or(TrafficStats {
                inbound_packets: 0,
                inbound_bytes: 0,
                outbound_packets: 0,
                outbound_bytes: 0,
            });

        stats.inbound_packets += 1;
        stats.inbound_bytes += packet_len;

        let _ = TRAFFIC_STATS.insert(&source_addr, &stats, 0);
    }

    // 统计出站流量 (目标IP是目标IP)
    if unsafe { TARGET_IP.get(&dest_addr) }.is_some() {
        info!(
            &ctx,
            "OUTBOUND - DST IP: {:i}, SIZE: {} bytes", dest_addr, packet_len
        );

        // 更新出站流量统计
        let mut stats = unsafe { TRAFFIC_STATS.get(&dest_addr) }
            .copied()
            .unwrap_or(TrafficStats {
                inbound_packets: 0,
                inbound_bytes: 0,
                outbound_packets: 0,
                outbound_bytes: 0,
            });

        stats.outbound_packets += 1;
        stats.outbound_bytes += packet_len;

        let _ = TRAFFIC_STATS.insert(&dest_addr, &stats, 0);
    }

    Ok(xdp_action::XDP_PASS)
}
