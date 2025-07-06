#![no_std]

pub mod utils;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PacketLog {
    pub ipv4_address: u32,
    pub action: u32,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {} // (1)

// 流量统计结构，与eBPF程序中的结构保持一致
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrafficStats {
    pub inbound_packets: u64,
    pub inbound_bytes: u64,
    pub outbound_packets: u64,
    pub outbound_bytes: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for TrafficStats {}

impl TrafficStats {
    pub fn new() -> Self {
        Self {
            inbound_packets: 0,
            inbound_bytes: 0,
            outbound_packets: 0,
            outbound_bytes: 0,
        }
    }

    pub fn total_packets(&self) -> u64 {
        self.inbound_packets + self.outbound_packets
    }

    pub fn total_bytes(&self) -> u64 {
        self.inbound_bytes + self.outbound_bytes
    }
}
