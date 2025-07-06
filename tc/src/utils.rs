use std::net::Ipv4Addr;

pub fn u32_to_ip(ip: u32) -> Ipv4Addr {
    Ipv4Addr::from(ip)
}
