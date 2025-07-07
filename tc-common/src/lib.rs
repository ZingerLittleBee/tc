#![no_std]

pub mod utils;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PacketLog {
    pub ipv4_address: u32,
    pub action: u32,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {}

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

// === 多维度监控数据结构 ===

// 网络流唯一标识键
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FlowKey {
    pub ip: u32,       // IP 地址
    pub port: u16,     // 端口号
    pub protocol: u8,  // 协议类型：6=TCP, 17=UDP
    pub direction: u8, // 方向：0=inbound, 1=outbound
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for FlowKey {}

// 增强的流量统计结构
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EnhancedTrafficStats {
    pub inbound_packets: u64,
    pub inbound_bytes: u64,
    pub outbound_packets: u64,
    pub outbound_bytes: u64,
    pub protocol: u8,          // 协议类型
    pub last_seen: u64,        // 最后活跃时间（纳秒时间戳）
    pub connection_count: u32, // 连接数（对于相同 IP+Port 的统计）
    pub _padding: u32,         // 对齐填充
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for EnhancedTrafficStats {}

impl EnhancedTrafficStats {
    pub fn new(protocol: u8) -> Self {
        Self {
            inbound_packets: 0,
            inbound_bytes: 0,
            outbound_packets: 0,
            outbound_bytes: 0,
            protocol,
            last_seen: 0,
            connection_count: 0,
            _padding: 0,
        }
    }

    pub fn total_packets(&self) -> u64 {
        self.inbound_packets + self.outbound_packets
    }

    pub fn total_bytes(&self) -> u64 {
        self.inbound_bytes + self.outbound_bytes
    }
}

// 协议统计结构
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProtocolStats {
    pub tcp_flows: u32,   // TCP 流数量
    pub udp_flows: u32,   // UDP 流数量
    pub tcp_bytes: u64,   // TCP 字节数
    pub udp_bytes: u64,   // UDP 字节数
    pub tcp_packets: u64, // TCP 包数量
    pub udp_packets: u64, // UDP 包数量
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ProtocolStats {}

impl ProtocolStats {
    pub fn new() -> Self {
        Self {
            tcp_flows: 0,
            udp_flows: 0,
            tcp_bytes: 0,
            udp_bytes: 0,
            tcp_packets: 0,
            udp_packets: 0,
        }
    }

    pub fn total_flows(&self) -> u32 {
        self.tcp_flows + self.udp_flows
    }

    pub fn total_bytes(&self) -> u64 {
        self.tcp_bytes + self.udp_bytes
    }

    pub fn total_packets(&self) -> u64 {
        self.tcp_packets + self.udp_packets
    }
}

// 端口统计结构
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PortStats {
    pub port: u16,               // 端口号
    pub protocol: u8,            // 主要协议类型
    pub _padding: u8,            // 对齐填充
    pub total_bytes: u64,        // 总字节数
    pub total_packets: u64,      // 总包数
    pub active_connections: u32, // 活跃连接数
    pub last_active: u64,        // 最后活跃时间
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PortStats {}

impl PortStats {
    pub fn new(port: u16, protocol: u8) -> Self {
        Self {
            port,
            protocol,
            _padding: 0,
            total_bytes: 0,
            total_packets: 0,
            active_connections: 0,
            last_active: 0,
        }
    }
}

// 协议常量
pub const PROTOCOL_TCP: u8 = 6;
pub const PROTOCOL_UDP: u8 = 17;

// 方向常量
pub const DIRECTION_INBOUND: u8 = 0;
pub const DIRECTION_OUTBOUND: u8 = 1;
