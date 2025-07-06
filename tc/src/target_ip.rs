use std::env;

use tc_common::utils::ip_to_u32;

use crate::utils::u32_to_ip;

#[derive(Debug, Clone, Copy)]
pub struct TargetIp(pub u32);

impl TargetIp {
    pub fn to_string(&self) -> String {
        u32_to_ip(self.0).to_string()
    }
}

pub fn get_target_ip() -> anyhow::Result<Vec<TargetIp>> {
    let target_ip_str = env::var("TARGET_IP").unwrap_or_default();

    let target_ip: Vec<&str> = target_ip_str.split(',').collect();

    let target_ip_u32: Vec<u32> = target_ip
        .iter()
        .map(|ip| {
            ip_to_u32(
                ip.split('.')
                    .map(|s| s.parse().unwrap())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap(),
            )
        })
        .collect();

    Ok(target_ip_u32.iter().map(|ip| TargetIp(*ip)).collect())
}
