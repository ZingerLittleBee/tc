use serde::{Deserialize, Serialize};
use tc_common::{EnhancedTrafficStats, FlowKey, PortStats, ProtocolStats};

// 可序列化的 FlowKey 包装
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct SerializableFlowKey {
    pub ip: u32,
    pub port: u16,
    pub protocol: u8,
    pub direction: u8,
}

impl From<FlowKey> for SerializableFlowKey {
    fn from(flow_key: FlowKey) -> Self {
        Self {
            ip: flow_key.ip,
            port: flow_key.port,
            protocol: flow_key.protocol,
            direction: flow_key.direction,
        }
    }
}

impl Into<FlowKey> for SerializableFlowKey {
    fn into(self) -> FlowKey {
        FlowKey {
            ip: self.ip,
            port: self.port,
            protocol: self.protocol,
            direction: self.direction,
        }
    }
}

// 可序列化的 EnhancedTrafficStats 包装
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableEnhancedTrafficStats {
    pub inbound_packets: u64,
    pub inbound_bytes: u64,
    pub outbound_packets: u64,
    pub outbound_bytes: u64,
    pub protocol: u8,
    pub last_seen: u64,
    pub connection_count: u32,
}

impl From<EnhancedTrafficStats> for SerializableEnhancedTrafficStats {
    fn from(stats: EnhancedTrafficStats) -> Self {
        Self {
            inbound_packets: stats.inbound_packets,
            inbound_bytes: stats.inbound_bytes,
            outbound_packets: stats.outbound_packets,
            outbound_bytes: stats.outbound_bytes,
            protocol: stats.protocol,
            last_seen: stats.last_seen,
            connection_count: stats.connection_count,
        }
    }
}

impl Into<EnhancedTrafficStats> for SerializableEnhancedTrafficStats {
    fn into(self) -> EnhancedTrafficStats {
        EnhancedTrafficStats {
            inbound_packets: self.inbound_packets,
            inbound_bytes: self.inbound_bytes,
            outbound_packets: self.outbound_packets,
            outbound_bytes: self.outbound_bytes,
            protocol: self.protocol,
            last_seen: self.last_seen,
            connection_count: self.connection_count,
            _padding: 0,
        }
    }
}

impl SerializableEnhancedTrafficStats {
    pub fn total_packets(&self) -> u64 {
        self.inbound_packets + self.outbound_packets
    }

    pub fn total_bytes(&self) -> u64 {
        self.inbound_bytes + self.outbound_bytes
    }
}

// 可序列化的 ProtocolStats 包装
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableProtocolStats {
    pub tcp_flows: u32,
    pub udp_flows: u32,
    pub tcp_bytes: u64,
    pub udp_bytes: u64,
    pub tcp_packets: u64,
    pub udp_packets: u64,
}

impl From<ProtocolStats> for SerializableProtocolStats {
    fn from(stats: ProtocolStats) -> Self {
        Self {
            tcp_flows: stats.tcp_flows,
            udp_flows: stats.udp_flows,
            tcp_bytes: stats.tcp_bytes,
            udp_bytes: stats.udp_bytes,
            tcp_packets: stats.tcp_packets,
            udp_packets: stats.udp_packets,
        }
    }
}

impl Into<ProtocolStats> for SerializableProtocolStats {
    fn into(self) -> ProtocolStats {
        ProtocolStats {
            tcp_flows: self.tcp_flows,
            udp_flows: self.udp_flows,
            tcp_bytes: self.tcp_bytes,
            udp_bytes: self.udp_bytes,
            tcp_packets: self.tcp_packets,
            udp_packets: self.udp_packets,
        }
    }
}

impl SerializableProtocolStats {
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

// 可序列化的 PortStats 包装
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializablePortStats {
    pub port: u16,
    pub protocol: u8,
    pub total_bytes: u64,
    pub total_packets: u64,
    pub active_connections: u32,
    pub last_active: u64,
}

impl From<PortStats> for SerializablePortStats {
    fn from(stats: PortStats) -> Self {
        Self {
            port: stats.port,
            protocol: stats.protocol,
            total_bytes: stats.total_bytes,
            total_packets: stats.total_packets,
            active_connections: stats.active_connections,
            last_active: stats.last_active,
        }
    }
}

impl Into<PortStats> for SerializablePortStats {
    fn into(self) -> PortStats {
        PortStats {
            port: self.port,
            protocol: self.protocol,
            _padding: 0,
            total_bytes: self.total_bytes,
            total_packets: self.total_packets,
            active_connections: self.active_connections,
            last_active: self.last_active,
        }
    }
}

impl SerializablePortStats {
    pub fn new(port: u16, protocol: u8) -> Self {
        Self {
            port,
            protocol,
            total_bytes: 0,
            total_packets: 0,
            active_connections: 0,
            last_active: 0,
        }
    }
} 