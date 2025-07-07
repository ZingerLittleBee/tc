use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rocksdb::{DBCompressionType, IteratorMode, Options, WriteBatch, DB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tc_common::{EnhancedTrafficStats, FlowKey, PortStats, ProtocolStats};
use crate::serializable_types::{
    SerializableEnhancedTrafficStats, SerializableFlowKey, SerializablePortStats, SerializableProtocolStats
};

// 时序数据记录
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowRecord {
    pub timestamp: DateTime<Utc>,
    pub flow_key: SerializableFlowKey,
    pub stats: SerializableEnhancedTrafficStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolRecord {
    pub timestamp: DateTime<Utc>,
    pub ip: u32,
    pub stats: SerializableProtocolStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortRecord {
    pub timestamp: DateTime<Utc>,
    pub port: u16,
    pub stats: SerializablePortStats,
}

// 存储层主要结构
pub struct TrafficStorage {
    db: DB,
}

impl TrafficStorage {
    pub fn new(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        // 针对时序数据优化配置
        opts.set_compression_type(DBCompressionType::Lz4);
        opts.set_write_buffer_size(32 * 1024 * 1024); // 32MB
        opts.set_max_write_buffer_number(3);
        opts.set_level_zero_file_num_compaction_trigger(8);
        opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB

        // 针对读性能优化
        opts.set_max_open_files(1000);
        opts.set_use_fsync(false);

        let db = DB::open(&opts, path)
            .with_context(|| format!("Failed to open RocksDB at path: {}", path))?;

        Ok(TrafficStorage { db })
    }

    // 批量存储流量数据快照
    pub fn store_traffic_snapshot(
        &self,
        flows: &HashMap<FlowKey, EnhancedTrafficStats>,
        protocols: &HashMap<u32, ProtocolStats>,
        ports: &HashMap<u16, PortStats>,
    ) -> Result<()> {
        let mut batch = WriteBatch::default();
        let timestamp = Utc::now();
        let ts = timestamp.timestamp();

        // 存储流量数据 - 键格式: "flow:{timestamp}:{ip}:{port}:{protocol}:{direction}"
        for (flow_key, stats) in flows {
            let record = FlowRecord {
                timestamp,
                flow_key: (*flow_key).into(),
                stats: (*stats).into(),
            };

            let key = format!(
                "flow:{:010}:{}:{}:{}:{}",
                ts, flow_key.ip, flow_key.port, flow_key.protocol, flow_key.direction
            );
            let value = bincode::serialize(&record)?;
            batch.put(key.as_bytes(), &value);

            // 额外索引：按 IP 查询 - "ip_flows:{ip}:{timestamp}:{port}:{protocol}:{direction}"
            let ip_key = format!(
                "ip_flows:{}:{:010}:{}:{}:{}",
                flow_key.ip, ts, flow_key.port, flow_key.protocol, flow_key.direction
            );
            batch.put(ip_key.as_bytes(), &value);

            // 额外索引：按端口查询 - "port_flows:{port}:{timestamp}:{ip}:{protocol}:{direction}"
            let port_key = format!(
                "port_flows:{}:{:010}:{}:{}:{}",
                flow_key.port, ts, flow_key.ip, flow_key.protocol, flow_key.direction
            );
            batch.put(port_key.as_bytes(), &value);
        }

        // 存储协议统计 - 键格式: "protocol:{timestamp}:{ip}"
        for (ip, stats) in protocols {
            let record = ProtocolRecord {
                timestamp,
                ip: *ip,
                stats: (*stats).into(),
            };

            let key = format!("protocol:{:010}:{}", ts, ip);
            let value = bincode::serialize(&record)?;
            batch.put(key.as_bytes(), &value);

            // 按 IP 索引
            let ip_proto_key = format!("ip_protocol:{}:{:010}", ip, ts);
            batch.put(ip_proto_key.as_bytes(), &value);
        }

        // 存储端口统计 - 键格式: "port_stats:{timestamp}:{port}"
        for (port, stats) in ports {
            let record = PortRecord {
                timestamp,
                port: *port,
                stats: (*stats).into(),
            };

            let key = format!("port_stats:{:010}:{}", ts, port);
            let value = bincode::serialize(&record)?;
            batch.put(key.as_bytes(), &value);
        }

        self.db.write(batch)?;
        Ok(())
    }

    // 查询指定 IP 的历史流量数据
    pub fn get_ip_flows_history(
        &self,
        ip: u32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<FlowRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let prefix = format!("ip_flows:{}:", ip);
        let start_key = format!("ip_flows:{}:{:010}:", ip, start_ts);
        let end_key = format!("ip_flows:{}:{:010}:", ip, end_ts);

        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with(&prefix) || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<FlowRecord>(&value) {
                results.push(record);
            }
        }

        Ok(results)
    }

    // 查询热门端口统计
    pub fn get_top_ports(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<PortRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let start_key = format!("port_stats:{:010}:", start_ts);
        let end_key = format!("port_stats:{:010}:", end_ts);

        let mut port_aggregates: HashMap<u16, SerializablePortStats> = HashMap::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with("port_stats:") || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<PortRecord>(&value) {
                let entry = port_aggregates
                    .entry(record.port)
                    .or_insert_with(|| SerializablePortStats::new(record.port, record.stats.protocol));

                entry.total_bytes += record.stats.total_bytes;
                entry.total_packets += record.stats.total_packets;
                entry.active_connections += record.stats.active_connections;
                entry.last_active = entry.last_active.max(record.stats.last_active);
            }
        }

        // 按总字节数排序并取前 N 个
        let mut sorted_ports: Vec<_> = port_aggregates
            .into_iter()
            .map(|(port, stats)| PortRecord {
                timestamp: end_time,
                port,
                stats,
            })
            .collect();

        sorted_ports.sort_by(|a, b| b.stats.total_bytes.cmp(&a.stats.total_bytes));
        sorted_ports.truncate(limit);

        Ok(sorted_ports)
    }

    // 查询指定时间范围的协议统计
    pub fn get_protocol_stats_history(
        &self,
        ip: u32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ProtocolRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let prefix = format!("ip_protocol:{}:", ip);
        let start_key = format!("ip_protocol:{}:{:010}", ip, start_ts);
        let end_key = format!("ip_protocol:{}:{:010}", ip, end_ts);

        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with(&prefix) || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<ProtocolRecord>(&value) {
                results.push(record);
            }
        }

        Ok(results)
    }

    // 获取实时快照数据（最近的记录）
    pub fn get_latest_snapshot(
        &self,
    ) -> Result<(Vec<FlowRecord>, Vec<ProtocolRecord>, Vec<PortRecord>)> {
        let now = Utc::now();
        let start_time = now - chrono::Duration::minutes(1); // 最近1分钟的数据

        let flows = self.get_flows_in_timerange(start_time, now)?;
        let protocols = self.get_all_protocol_stats_in_timerange(start_time, now)?;
        let ports = self.get_all_port_stats_in_timerange(start_time, now)?;

        Ok((flows, protocols, ports))
    }

    // 内部辅助方法：获取时间范围内的所有流量数据
    fn get_flows_in_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<FlowRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let start_key = format!("flow:{:010}:", start_ts);
        let end_key = format!("flow:{:010}:", end_ts);

        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with("flow:") || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<FlowRecord>(&value) {
                results.push(record);
            }
        }

        Ok(results)
    }

    // 内部辅助方法：获取时间范围内的所有协议统计
    fn get_all_protocol_stats_in_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ProtocolRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let start_key = format!("protocol:{:010}:", start_ts);
        let end_key = format!("protocol:{:010}:", end_ts);

        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with("protocol:") || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<ProtocolRecord>(&value) {
                results.push(record);
            }
        }

        Ok(results)
    }

    // 内部辅助方法：获取时间范围内的所有端口统计
    fn get_all_port_stats_in_timerange(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<PortRecord>> {
        let start_ts = start_time.timestamp();
        let end_ts = end_time.timestamp();
        let start_key = format!("port_stats:{:010}:", start_ts);
        let end_key = format!("port_stats:{:010}:", end_ts);

        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(
            start_key.as_bytes(),
            rocksdb::Direction::Forward,
        ));

        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if !key_str.starts_with("port_stats:") || key_str.as_ref() > end_key.as_str() {
                break;
            }

            if let Ok(record) = bincode::deserialize::<PortRecord>(&value) {
                results.push(record);
            }
        }

        Ok(results)
    }

    // 数据清理：删除过期数据
    pub fn cleanup_old_data(&self, before: DateTime<Utc>) -> Result<usize> {
        let mut batch = WriteBatch::default();
        let before_ts = before.timestamp();
        let end_key = format!("flow:{:010}:", before_ts);
        let protocol_end_key = format!("protocol:{:010}:", before_ts);
        let port_end_key = format!("port_stats:{:010}:", before_ts);

        let mut deleted_count = 0;

        // 清理流量数据
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);

            if key_str.starts_with("flow:") && key_str.as_ref() <= end_key.as_str() {
                batch.delete(&key);
                deleted_count += 1;
            } else if key_str.starts_with("protocol:")
                && key_str.as_ref() <= protocol_end_key.as_str()
            {
                batch.delete(&key);
                deleted_count += 1;
            } else if key_str.starts_with("port_stats:")
                && key_str.as_ref() <= port_end_key.as_str()
            {
                batch.delete(&key);
                deleted_count += 1;
            }
        }

        if deleted_count > 0 {
            self.db.write(batch)?;
        }

        Ok(deleted_count)
    }
}
