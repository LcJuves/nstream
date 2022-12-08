use super::vtun::DEFAULT_FD;
use crate::{debug_println, seeval};

use core::ffi::{c_char, c_int, c_uchar, c_ulong};
use core::mem::{size_of, zeroed};
use std::ffi::CString;
use std::fmt::Debug;
use std::os::unix::prelude::{AsRawFd, RawFd};

use libc::{
    connect, ioctl, size_t, sockaddr, sockaddr_ctl, socket, socklen_t, strcpy, AF_SYSTEM,
    AF_SYS_CONTROL, PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL,
};

/// Kernel control names must be no longer than [MAX_KCTL_NAME].
pub const MAX_KCTL_NAME: size_t = 96;

/// Name registered by the utun kernel control
pub const UTUN_CONTROL_NAME: &'static str = "com.apple.net.utun_control";

/// The [CTLIOCGINFO] ioctl can be used to convert a kernel control name to a kernel control id.
pub const CTLIOCGINFO: c_ulong = 0xc0644e03;

/// This structure is used with the CTLIOCGINFO ioctl to
/// translate from a kernel control name to a control id.
#[derive(Debug)]
#[repr(C)]
pub struct CtlInfo {
    /// The kernel control id, filled out upon return.
    pub ctl_id: u32,
    /// The kernel control name to find.
    pub ctl_name: [c_char; MAX_KCTL_NAME],
}

impl CtlInfo {
    pub fn new_with(name: &str) -> Self {
        let cstr_name = CString::new(name).unwrap();
        if cstr_name.as_bytes_with_nul().len() > MAX_KCTL_NAME {
            panic!("`name` too long")
        }
        unsafe {
            let mut ctl_info = zeroed::<Self>();
            let cstr_ptr = cstr_name.into_raw() as *const c_char;
            strcpy(ctl_info.ctl_name.as_mut_ptr(), cstr_ptr);
            ctl_info
        }
    }
}

#[derive(Debug)]
pub struct UTun {
    fd: c_int,
}

impl UTun {
    // Open specific utun device unit and return fd.
    // If the unit number is already in use, return -1.
    #[inline]
    fn _open(uint: &u32) -> c_int {
        let mut sc = unsafe { zeroed::<sockaddr_ctl>() };
        let ctl_info = CtlInfo::new_with(UTUN_CONTROL_NAME);

        let fd = unsafe { socket(PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL) };
        if fd < 0 {
            debug_println!("socket(PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL)");
            return DEFAULT_FD;
        }
        if unsafe { ioctl(fd, CTLIOCGINFO, &ctl_info) } == -1 {
            debug_println!("ioctl(fd, CTLIOCGINFO, &ctl_info)");
            return DEFAULT_FD;
        }
        seeval!(ctl_info);

        let sockaddr_ctl_size = size_of::<sockaddr_ctl>();

        sc.sc_id = ctl_info.ctl_id;
        sc.sc_len = sockaddr_ctl_size as c_uchar;
        sc.sc_family = AF_SYSTEM as c_uchar;
        sc.ss_sysaddr = AF_SYS_CONTROL as u16;
        sc.sc_unit = uint + 1;
        sc.sc_reserved = unsafe { zeroed() };

        seeval!(sc.sc_id);
        seeval!(sc.sc_len);
        seeval!(sc.sc_family);
        seeval!(sc.ss_sysaddr);
        seeval!(sc.sc_unit);
        seeval!(sc.sc_reserved);

        let sockaddr = &sc as *const sockaddr_ctl as *const sockaddr;
        unsafe {
            seeval!((*sockaddr).sa_len);
            seeval!((*sockaddr).sa_family);
            seeval!((*sockaddr).sa_data);
        }

        // If the connect is successful, a utunX device will be created, where X
        // is our unit number - 1.
        if unsafe { connect(fd, sockaddr, sockaddr_ctl_size as socklen_t) } == -1 {
            return -1;
        }

        seeval!(fd);
        fd
    }

    pub fn new() -> Self {
        let mut fd: c_int = DEFAULT_FD;
        for unit in 0..256 {
            fd = Self::_open(&unit);
            if fd >= 0 {
                break;
            }
        }
        UTun { fd }
    }
}

impl AsRawFd for UTun {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}
