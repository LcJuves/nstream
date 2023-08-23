use crate::{Tun, UTun};

use core::ffi::{c_int, c_uint};

#[derive(Debug)]
pub struct VTun {
    fd: c_int,
}

impl Tun for VTun {
    fn new() -> Self {
        #[cfg(target_os = "macos")]
        VTun { fd: super::UTun::new().as_raw_fd() }
    }

    #[inline]
    fn ifname(&self) -> std::io::Result<String> {
        #[cfg(target_os = "macos")]
        return Into::<UTun>::into(self.fd).ifname();
        #[allow(unreachable_code)]
        Ok(String::from(""))
    }

    #[inline]
    fn ifindex(&self) -> std::io::Result<c_uint> {
        #[cfg(target_os = "macos")]
        return Into::<UTun>::into(self.fd).ifindex();
        #[allow(unreachable_code)]
        Ok(0)
    }

    #[inline]
    fn mtu(&self) -> std::io::Result<c_int> {
        #[cfg(target_os = "macos")]
        return Into::<UTun>::into(self.fd).mtu();
        #[allow(unreachable_code)]
        Ok(0)
    }

    #[inline]
    fn set_mtu(&self, n: c_int) -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        return Into::<UTun>::into(self.fd).set_mtu(n);
        #[allow(unreachable_code)]
        Ok(())
    }

    #[inline]
    fn config_with(&self, conf: crate::VTunConfig) -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        return Into::<UTun>::into(self.fd).config_with(conf);
        #[allow(unreachable_code)]
        Ok(())
    }
}

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
impl AsRawFd for VTun {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.fd
    }
}
