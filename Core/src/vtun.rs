use core::ffi::c_int;

use std::os::unix::prelude::{AsRawFd, RawFd};

#[derive(Debug)]
pub struct VTun {
    fd: c_int,
}

pub const DEFAULT_FD: c_int = -1;

impl VTun {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        VTun { fd: super::UTun::new().as_raw_fd() }
    }
}

impl AsRawFd for VTun {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}
