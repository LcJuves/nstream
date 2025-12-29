use crate::{Tun, VTunConfig, debug_println, seeval, set_cloexec, set_nonblock};

use core::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void};
use core::mem::{size_of, size_of_val, transmute, zeroed};
use std::ffi::CString;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;

use libc::{
    AF_INET, AF_SYS_CONTROL, AF_SYSTEM, CTLIOCGINFO, IFF_UP, IFNAMSIZ, MAX_KCTL_NAME, PF_SYSTEM,
    SOCK_DGRAM, SYSPROTO_CONTROL, c_short, close, connect, ctl_info, freeifaddrs, getifaddrs,
    if_nametoindex, ifaddrs, in_addr_t, ioctl, sa_family_t, sockaddr, sockaddr_ctl, sockaddr_in,
    sockaddr_in6, socket, socklen_t, strcpy,
};

/// Name registered by the utun kernel control
pub const UTUN_CONTROL_NAME: &'static str = "com.apple.net.utun_control";
pub const SIOCGIFMTU: c_ulong = 0xc0206933; /* get IF mtu */
pub const SIOCSIFMTU: c_ulong = 0x80206934; /* set IF mtu */
pub const SIOCGIFCONF: c_ulong = 0xc00c6924; /* get ifnet list */
pub const SIOCSIFADDR: c_ulong = 0x8020690c; /* set ifnet address */
pub const SIOCSIFFLAGS: c_ulong = 0x80206910; /* set ifnet flags */
pub const SIOCGIFFLAGS: c_ulong = 0xc0206911; /* get ifnet flags */
pub const SIOCSIFNETMASK: c_ulong = 0x80206916; /* set net addr mask */
/// The maximum number of interfaces
pub const MAX_IF_NUM: usize = 16;

pub fn new_ctl_info_with(name: &str) -> Result<ctl_info> {
    let cstr_name = CString::new(name).unwrap();
    if size_of_val(&cstr_name) > MAX_KCTL_NAME {
        Err(Error::new(ErrorKind::InvalidData, "`name` too long"))
    } else {
        unsafe {
            let mut ctl_info = zeroed::<ctl_info>();
            let cstr_ptr = cstr_name.into_raw() as *const c_char;
            strcpy(ctl_info.ctl_name.as_mut_ptr(), cstr_ptr);
            Ok(ctl_info)
        }
    }
}

#[allow(non_camel_case_types)]
pub type caddr_t = *mut c_char;

/*
 * ifdevmtu: interface device mtu
 *    Used with SIOCGIFDEVMTU to get the current mtu in use by the device,
 *    as well as the minimum and maximum mtu allowed by the device.
 */
#[repr(C)]
#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct ifdevmtu {
    pub ifdm_current: c_int,
    pub ifdm_min: c_int,
    pub ifdm_max: c_int,
}

impl Copy for ifdevmtu {}

#[repr(C)]
#[derive(Clone)]
pub union ifk_data {
    pub ifk_ptr: *mut c_void,
    pub ifk_value: c_int,
}

impl Copy for ifk_data {}

/*
 *  ifkpi: interface kpi ioctl
 *  Used with SIOCSIFKPI and SIOCGIFKPI.
 *
 *  ifk_module_id - From in the kernel, a value from kev_vendor_code_find. From
 *       user space, a value from SIOCGKEVVENDOR ioctl on a kernel event socket.
 *  ifk_type - The type. Types are specific to each module id.
 *  ifk_data - The data. ifk_ptr may be a 64bit pointer for 64 bit processes.
 *
 *  Copying data between user space and kernel space is done using copyin
 *  and copyout. A process may be running in 64bit mode. In such a case,
 *  the pointer will be a 64bit pointer, not a 32bit pointer. The following
 *  sample is a safe way to copy the data in to the kernel from either a
 *  32bit or 64bit process:
 *
 *  user_addr_t tmp_ptr;
 *  if (IS_64BIT_PROCESS(current_proc())) {
 *       tmp_ptr = CAST_USER_ADDR_T(ifkpi.ifk_data.ifk_ptr64);
 *  }
 *  else {
 *       tmp_ptr = CAST_USER_ADDR_T(ifkpi.ifk_data.ifk_ptr);
 *  }
 *  error = copyin(tmp_ptr, allocated_dst_buffer, size of allocated_dst_buffer);
 */
#[repr(C)]
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub struct ifkpi {
    pub ifk_module_id: c_uint,
    pub ifk_type: c_uint,
    pub ifk_data: ifk_data,
}

impl Copy for ifkpi {}

#[repr(C)]
#[derive(Clone)]
pub union ifr_ifru {
    pub ifru_addr: sockaddr,
    pub ifru_dstaddr: sockaddr,
    pub ifru_broadaddr: sockaddr,
    pub ifru_flags: c_short,
    pub ifru_metric: c_int,
    pub ifru_mtu: c_int,
    pub ifru_phys: c_int,
    pub ifru_media: c_int,
    pub ifru_intval: c_int,
    pub ifru_data: caddr_t,
    pub ifru_devmtu: ifdevmtu,
    pub ifru_kpi: ifkpi,
    pub ifru_wake_flags: u32,
    pub ifru_route_refcnt: u32,
    pub ifru_cap: [c_int; 2],
    pub ifru_functional_type: u32,
}

impl Copy for ifr_ifru {}

#[repr(C)]
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub struct ifreq {
    pub ifr_name: [c_char; IFNAMSIZ],
    pub ifr_ifru: ifr_ifru,
}

impl Copy for ifreq {}

#[derive(Debug)]
pub struct UTun {
    fd: c_int,
}

impl UTun {
    /// Helper functions that tries to open utun device
    /// return -2 on early initialization failures (utun not supported
    /// at all (old OS X) and -1 on initlization failure of utun
    /// device (utun works but utunX is already used
    pub fn open_utun(utunnum: &c_uint) -> c_int {
        let ctl_info = new_ctl_info_with(UTUN_CONTROL_NAME).unwrap();
        let mut sc = unsafe { zeroed::<sockaddr_ctl>() };

        let fd: c_int = unsafe { socket(PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL) };
        if fd < 0 {
            debug_println!("Opening utun{} failed (socket(SYSPROTO_CONTROL))", utunnum);
            return -2;
        }
        if unsafe { ioctl(fd, CTLIOCGINFO, &ctl_info) } == -1 {
            debug_println!("Opening utun{} failed (ioctl(CTLIOCGINFO))", utunnum);
            unsafe { close(fd) };
            return -2;
        }
        seeval!(&ctl_info);

        let sockaddr_ctl_size = size_of::<sockaddr_ctl>();
        sc.sc_id = ctl_info.ctl_id;
        sc.sc_len = sockaddr_ctl_size as c_uchar;
        sc.sc_family = AF_SYSTEM as c_uchar;
        sc.ss_sysaddr = AF_SYS_CONTROL as u16;
        sc.sc_unit = utunnum + 1;
        seeval!(&sc);

        let sockaddr = &sc as *const sockaddr_ctl as *const sockaddr;
        unsafe { seeval!(*sockaddr) };

        /* If the connect is successful, a utunX device will be created, where X
         * is (sc.sc_unit - 1) */
        if unsafe { connect(fd, sockaddr, sockaddr_ctl_size as socklen_t) } < 0 {
            debug_println!("Opening utun{} failed (connect(AF_SYS_CONTROL))", utunnum);
            unsafe { close(fd) };
            return -1;
        }

        set_nonblock(fd);
        set_cloexec(fd); /* don't pass fd to scripts */

        fd
    }
}

impl Tun for UTun {
    #[inline]
    fn new() -> Self {
        let mut fd: c_int = c_int::default();
        for utunnum in 0..255 {
            fd = Self::open_utun(&utunnum);
            /* Break if the fd is valid,
             * or if early initialization failed (-2) */
            if fd != -1 {
                break;
            }
        }
        UTun { fd }
    }

    fn ifname(&self) -> Result<String> {
        unsafe extern "C" {
            fn utun_ifname(name: *mut c_char, fd: c_int) -> c_int;
        }

        let mut utunname: [c_char; IFNAMSIZ] = unsafe { zeroed() };
        if unsafe { utun_ifname(utunname.as_mut_ptr(), self.fd) } != 0 {
            return Err(Error::last_os_error());
        }
        let utunname = unsafe { std::ffi::CStr::from_ptr(utunname.as_ptr()) };
        let utunname = utunname.to_string_lossy().to_string();
        Ok(utunname)
    }

    fn config_with(&self, conf: VTunConfig) -> Result<()> {
        let sockfd: c_int = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
        if sockfd < 0 {
            set_cloexec(sockfd);
            return Err(Error::last_os_error());
        }

        let VTunConfig { mtu, ipv4, ipv6, netmask } = conf;
        let mut ifreq = unsafe { zeroed::<ifreq>() };
        let cstring_ifname = CString::new(self.ifname()?.as_str());
        let self_ifname_c_ptr = cstring_ifname.unwrap().into_raw();
        unsafe { strcpy(ifreq.ifr_name.as_mut_ptr(), self_ifname_c_ptr) };

        if let Some(mtu) = mtu {
            ifreq.ifr_ifru.ifru_mtu = mtu as c_int;
            if unsafe { ioctl(sockfd, SIOCSIFMTU, &mut ifreq) } < 0 {
                unsafe { close(sockfd) };
                return Err(Error::last_os_error());
            }
        }

        if unsafe { ioctl(sockfd, SIOCGIFFLAGS, &mut ifreq) } < 0 {
            unsafe { close(sockfd) };
            return Err(Error::last_os_error());
        }

        unsafe { ifreq.ifr_ifru.ifru_flags |= IFF_UP as c_short };
        if unsafe { ioctl(sockfd, SIOCSIFFLAGS, &mut ifreq) } < 0 {
            unsafe { close(sockfd) };
            return Err(Error::last_os_error());
        }

        if let Some(ipv4) = ipv4 {
            let mut sin = unsafe { zeroed::<sockaddr_in>() };
            sin.sin_family = AF_INET as sa_family_t;
            sin.sin_addr.s_addr = u32::from_ne_bytes(ipv4.octets()) as in_addr_t;

            ifreq.ifr_ifru.ifru_addr = unsafe { transmute::<sockaddr_in, sockaddr>(sin) };
            if unsafe { ioctl(sockfd, SIOCSIFADDR, &mut ifreq) } < 0 {
                unsafe { close(sockfd) };
                return Err(Error::last_os_error());
            }
        }

        if let Some(_ipv6) = ipv6 {
            todo!("IPV6 support for utunX device")
            /* let mut sin6 = unsafe { zeroed::<sockaddr_in6>() };
            sin6.sin6_family = AF_INET6 as sa_family_t;
            sin6.sin6_addr.s6_addr = ipv6.octets();

            ifreq.ifr_ifru.ifru_addr = unsafe { transmute_copy::<sockaddr_in6, sockaddr>(&sin6) };
            if unsafe { ioctl(sockfd, SIOCSIFADDR, &mut ifreq) } < 0 {
                unsafe { close(sockfd) };
                return Err(Error::last_os_error());
            } */
        }

        if let Some(netmask) = netmask {
            let mut sin = unsafe { zeroed::<sockaddr_in>() };
            sin.sin_family = AF_INET as sa_family_t;
            sin.sin_addr.s_addr = netmask;

            ifreq.ifr_ifru.ifru_addr = unsafe { transmute::<sockaddr_in, sockaddr>(sin) };
            if unsafe { ioctl(sockfd, SIOCSIFNETMASK, &mut ifreq) } < 0 {
                unsafe { close(sockfd) };
                return Err(Error::last_os_error());
            }
        }

        Ok(())
    }

    #[inline]
    fn ifindex(&self) -> Result<c_uint> {
        Ok(unsafe { if_nametoindex(self.ifname()?.as_ptr() as *const c_char) })
    }

    fn mtu(&self) -> Result<c_int> {
        let mut ifreq = unsafe { zeroed::<ifreq>() };
        let ifname = self.ifname()?;
        let self_ifname_c_ptr = CString::new(ifname.as_str()).unwrap().into_raw();
        unsafe { strcpy(ifreq.ifr_name.as_mut_ptr(), self_ifname_c_ptr) };
        if unsafe { ioctl(self.fd, SIOCGIFMTU, &mut ifreq) } == -1 {
            return Err(Error::last_os_error());
        }
        Ok(unsafe { ifreq.ifr_ifru.ifru_mtu })
    }

    fn set_mtu(&self, n: c_int) -> Result<()> {
        let mut ifreq = unsafe { zeroed::<ifreq>() };
        let ifname = self.ifname()?;
        let self_ifname_c_ptr = CString::new(ifname.as_str()).unwrap().into_raw();
        unsafe { strcpy(ifreq.ifr_name.as_mut_ptr(), self_ifname_c_ptr) };
        ifreq.ifr_ifru.ifru_mtu = n;

        if unsafe { ioctl(self.fd, SIOCSIFMTU, &mut ifreq) } == -1 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }
}

impl UTun {
    pub fn ifconf(&self) {
        // let sockfd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
        // if sockfd < 0 {}

        seeval!(size_of::<sockaddr>());
        seeval!(size_of::<sockaddr_in6>());
        seeval!(size_of::<SocketAddr>());

        unsafe {
            let mut ifap: *mut ifaddrs = core::ptr::null_mut();
            if getifaddrs(&mut ifap) == 0 {
                let mut ifa = ifap;
                while !ifa.is_null() {
                    let ifa_name = (*ifa).ifa_name;
                    let ifa_name_cstr = core::ffi::CStr::from_ptr(ifa_name);
                    let ifa_name_str = ifa_name_cstr.to_str().unwrap();
                    println!("Interface: {}", ifa_name_str);

                    ifa = (*ifa).ifa_next;
                }
                freeifaddrs(ifap);
            } else {
                println!("Failed to get network interface information");
            }
        }

        // let mut ifreq_arr = unsafe { zeroed::<[ifreq; 1]>() };
        // let mut ifreq = unsafe { zeroed::<ifreq>() };
        // let self_ifname = self.ifname();
        // let self_ifname_c_ptr = CString::new(self_ifname.unwrap().as_str()).unwrap().into_raw();
        // unsafe { strcpy(ifreq.ifr_name.as_mut_ptr(), self_ifname_c_ptr) };
        // ifreq_arr[0] = ifreq;

        // let mut ifconf = unsafe { zeroed::<ifconf>() };
        // let mut buf = unsafe { zeroed::<[c_char; 1024 * 10]>() };
        // ifconf.ifc_len = size_of_val(&buf) as c_int;
        // ifconf.ifc_ifcu.ifcu_buf = buf.as_mut_ptr();

        // if unsafe { ioctl(sockfd, SIOCGIFCONF, &mut ifconf) } == -1 {}
        // let tmp = unsafe { std::slice::from_raw_parts_mut(ifconf.ifc_ifcu.ifcu_req, 1) }.iter();
        // let ifreq_ptr = unsafe { ifconf.ifc_ifcu.ifcu_buf as *mut ifreq };
        // let if_num = ifconf.ifc_len as usize / size_of::<ifreq>();
        // seeval!(if_num);
        // for ifr in unsafe { std::slice::from_raw_parts_mut(ifreq_ptr, if_num) } {
        // seeval!(ifr.ifr_name);
        // }

        // for ifr in unsafe { std::slice::from_raw_parts_mut(ifconf.ifc_ifcu.ifcu_req, 1) }.iter() {
        //     unsafe { seeval!(ifr.ifr_ifru.ifru_mtu) };
        // }
        // seeval!(tmp);

        // unsafe { utun_ifconf(self.fd) };

        // const BUF_LEN: usize = 512;
        // let mut buf = unsafe { zeroed::<[c_char; BUF_LEN]>() };
        // ifconf.ifc_len = BUF_LEN as c_int;
        // ifconf.ifc_ifcu.ifcu_buf = buf.as_mut_ptr();

        // let sockfd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
        // if sockfd < 0 {
        //     debug_println!("sockfd < 0");
        //     // debug_println!("Opening utun{} failed (socket(SYSPROTO_CONTROL))", utunnum);
        //     // return -2;
        // }

        // let ifconf_ptr = unsafe { transmute::<*mut ifconf, *mut c_char>(&mut ifconf) };
        // let ret = unsafe { ioctl(self.fd, SIOCGIFCONF, &mut ifconf) };
        // let error = Error::last_os_error();
        // seeval!(error);
        // seeval!(ret);
        // seeval!(ifconf.ifc_len);
        // for _ifreq in ifreq {
        //     seeval!(_ifreq.ifr_name);
        // }
    }
}

impl From<c_int> for UTun {
    fn from(fd: c_int) -> Self {
        Self { fd }
    }
}

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
impl AsRawFd for UTun {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.fd
    }
}

#[cfg(unix)]
use std::os::fd::FromRawFd;
#[cfg(unix)]
impl FromRawFd for UTun {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        Self { fd }
    }
}
