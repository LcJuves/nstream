use core::ffi::{c_int, c_uint};
use std::io::Result;

use crate::VTunConfig;

pub trait Tun {
    fn new() -> Self;
    fn ifname(&self) -> Result<String>;
    fn config_with(&self, conf: VTunConfig) -> Result<()>;
    fn ifindex(&self) -> Result<c_uint>;
    fn mtu(&self) -> Result<c_int>;
    fn set_mtu(&self, n: c_int) -> Result<()>;
}
