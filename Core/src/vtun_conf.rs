use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy)]
pub struct VTunConfig {
    pub mtu: Option<u16>,
    pub ipv4_addr: Option<Ipv4Addr>,
    pub ipv6_addr: Option<Ipv6Addr>,
    pub netmask: Option<u32>,
}

impl Default for VTunConfig {
    fn default() -> Self {
        Self { mtu: None, ipv4_addr: None, ipv6_addr: None, netmask: None }
    }
}
