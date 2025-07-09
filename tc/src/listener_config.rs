use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 监听配置管理器
#[derive(Debug, Clone)]
pub struct ListenerConfig {
    /// 监听的 IP 地址列表
    pub listen_ips: Arc<RwLock<HashSet<u32>>>,
    /// 监听的端口列表
    pub listen_ports: Arc<RwLock<HashSet<u16>>>,
    /// 网络接口名称
    pub interface: Arc<RwLock<String>>,
}

/// API 请求结构 - 添加监听 IP
#[derive(Debug, Deserialize)]
pub struct AddListenerIpRequest {
    pub ip: String,
}

/// API 请求结构 - 添加监听端口
#[derive(Debug, Deserialize)]
pub struct AddListenerPortRequest {
    pub port: u16,
}

/// API 请求结构 - 移除监听 IP
#[derive(Debug, Deserialize)]
pub struct RemoveListenerIpRequest {
    pub ip: String,
}

/// API 请求结构 - 移除监听端口
#[derive(Debug, Deserialize)]
pub struct RemoveListenerPortRequest {
    pub port: u16,
}

/// API 响应结构 - 当前监听配置
#[derive(Debug, Serialize)]
pub struct ListenerConfigResponse {
    pub listen_ips: Vec<String>,
    pub listen_ports: Vec<u16>,
    pub interface: String,
}

/// 监听配置操作结果
#[derive(Debug, Serialize)]
pub struct ListenerOperationResult {
    pub success: bool,
    pub message: String,
    pub affected_item: Option<String>,
}

impl ListenerConfig {
    /// 创建新的监听配置管理器
    pub fn new(interface: String) -> Self {
        Self {
            listen_ips: Arc::new(RwLock::new(HashSet::new())),
            listen_ports: Arc::new(RwLock::new(HashSet::new())),
            interface: Arc::new(RwLock::new(interface)),
        }
    }

    /// 从现有的目标 IP 列表初始化
    pub async fn from_target_ips(interface: String, target_ips: Vec<u32>) -> Self {
        let config = Self::new(interface);
        {
            let mut ips = config.listen_ips.write().await;
            for ip in target_ips {
                ips.insert(ip);
            }
        }
        config
    }

    /// 添加监听 IP 地址
    pub async fn add_listen_ip(&self, ip_str: &str) -> Result<ListenerOperationResult> {
        let ip = self.parse_ip(ip_str)?;
        let mut ips = self.listen_ips.write().await;
        
        if ips.contains(&ip) {
            return Ok(ListenerOperationResult {
                success: false,
                message: format!("IP 地址 {} 已经在监听列表中", ip_str),
                affected_item: Some(ip_str.to_string()),
            });
        }

        ips.insert(ip);
        Ok(ListenerOperationResult {
            success: true,
            message: format!("成功添加监听 IP 地址: {}", ip_str),
            affected_item: Some(ip_str.to_string()),
        })
    }

    /// 移除监听 IP 地址
    pub async fn remove_listen_ip(&self, ip_str: &str) -> Result<ListenerOperationResult> {
        let ip = self.parse_ip(ip_str)?;
        let mut ips = self.listen_ips.write().await;
        
        if !ips.contains(&ip) {
            return Ok(ListenerOperationResult {
                success: false,
                message: format!("IP 地址 {} 不在监听列表中", ip_str),
                affected_item: Some(ip_str.to_string()),
            });
        }

        ips.remove(&ip);
        Ok(ListenerOperationResult {
            success: true,
            message: format!("成功移除监听 IP 地址: {}", ip_str),
            affected_item: Some(ip_str.to_string()),
        })
    }

    /// 添加监听端口
    pub async fn add_listen_port(&self, port: u16) -> Result<ListenerOperationResult> {
        if port == 0 {
            return Err(anyhow!("端口号不能为 0"));
        }

        let mut ports = self.listen_ports.write().await;
        
        if ports.contains(&port) {
            return Ok(ListenerOperationResult {
                success: false,
                message: format!("端口 {} 已经在监听列表中", port),
                affected_item: Some(port.to_string()),
            });
        }

        ports.insert(port);
        Ok(ListenerOperationResult {
            success: true,
            message: format!("成功添加监听端口: {}", port),
            affected_item: Some(port.to_string()),
        })
    }

    /// 移除监听端口
    pub async fn remove_listen_port(&self, port: u16) -> Result<ListenerOperationResult> {
        let mut ports = self.listen_ports.write().await;
        
        if !ports.contains(&port) {
            return Ok(ListenerOperationResult {
                success: false,
                message: format!("端口 {} 不在监听列表中", port),
                affected_item: Some(port.to_string()),
            });
        }

        ports.remove(&port);
        Ok(ListenerOperationResult {
            success: true,
            message: format!("成功移除监听端口: {}", port),
            affected_item: Some(port.to_string()),
        })
    }

    /// 获取当前监听配置
    pub async fn get_config(&self) -> ListenerConfigResponse {
        let ips = self.listen_ips.read().await;
        let ports = self.listen_ports.read().await;
        let interface = self.interface.read().await;

        let listen_ips: Vec<String> = ips
            .iter()
            .map(|&ip| self.u32_to_ip_string(ip))
            .collect();

        let listen_ports: Vec<u16> = ports.iter().cloned().collect();

        ListenerConfigResponse {
            listen_ips,
            listen_ports,
            interface: interface.clone(),
        }
    }

    /// 获取当前监听的 IP 地址列表（u32 格式）
    pub async fn get_listen_ips_u32(&self) -> Vec<u32> {
        let ips = self.listen_ips.read().await;
        ips.iter().cloned().collect()
    }

    /// 获取当前监听的端口列表
    pub async fn get_listen_ports(&self) -> Vec<u16> {
        let ports = self.listen_ports.read().await;
        ports.iter().cloned().collect()
    }

    /// 设置网络接口
    pub async fn set_interface(&self, interface: String) -> Result<()> {
        let mut iface = self.interface.write().await;
        *iface = interface;
        Ok(())
    }

    /// 获取网络接口
    pub async fn get_interface(&self) -> String {
        let interface = self.interface.read().await;
        interface.clone()
    }

    /// 解析 IP 地址字符串为 u32
    fn parse_ip(&self, ip_str: &str) -> Result<u32> {
        let ip: Ipv4Addr = ip_str.parse()
            .map_err(|_| anyhow!("无效的 IP 地址格式: {}", ip_str))?;
        Ok(u32::from(ip))
    }

    /// 将 u32 格式的 IP 转换为字符串
    fn u32_to_ip_string(&self, ip: u32) -> String {
        Ipv4Addr::from(ip).to_string()
    }
}

/// 验证 IP 地址格式
pub fn validate_ip_address(ip: &str) -> Result<()> {
    ip.parse::<Ipv4Addr>()
        .map_err(|_| anyhow!("无效的 IP 地址格式: {}", ip))?;
    Ok(())
}

/// 验证端口范围
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(anyhow!("端口号不能为 0"));
    }
    if port > 65535 {
        return Err(anyhow!("端口号不能大于 65535"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_listener_config_basic_operations() {
        let config = ListenerConfig::new("eth0".to_string());

        // 测试添加 IP
        let result = config.add_listen_ip("192.168.1.100").await.unwrap();
        assert!(result.success);

        // 测试重复添加 IP
        let result = config.add_listen_ip("192.168.1.100").await.unwrap();
        assert!(!result.success);

        // 测试添加端口
        let result = config.add_listen_port(8080).await.unwrap();
        assert!(result.success);

        // 测试重复添加端口
        let result = config.add_listen_port(8080).await.unwrap();
        assert!(!result.success);

        // 测试获取配置
        let config_response = config.get_config().await;
        assert_eq!(config_response.listen_ips.len(), 1);
        assert_eq!(config_response.listen_ports.len(), 1);
        assert_eq!(config_response.interface, "eth0");
    }

    #[test]
    fn test_ip_validation() {
        assert!(validate_ip_address("192.168.1.1").is_ok());
        assert!(validate_ip_address("10.0.0.1").is_ok());
        assert!(validate_ip_address("invalid_ip").is_err());
        assert!(validate_ip_address("256.256.256.256").is_err());
    }

    #[test]
    fn test_port_validation() {
        assert!(validate_port(80).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok());
        assert!(validate_port(0).is_err());
    }
}
