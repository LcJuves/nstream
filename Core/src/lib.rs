#[cfg(target_os = "macos")]
mod utun;
#[cfg(target_os = "macos")]
pub use utun::*;

mod vtun;
pub use vtun::*;

mod tun;
pub use tun::*;

mod vtun_conf;
pub use vtun_conf::*;

use core::ffi::c_int;
use std::net::{IpAddr, UdpSocket};

use lazy_static::lazy_static;
use libc::{fcntl, FD_CLOEXEC, F_GETFL, F_SETFD, F_SETFL, O_NONBLOCK};
use maxminddb::{geoip2::Country, Reader};

lazy_static! {
    pub static ref GEOIP2_COUNTRY_MMDB_BUF: &'static [u8] = include_bytes!("../Country.mmdb");
}

pub fn set_nonblock(fd: c_int) -> c_int {
    let mut flag: c_int = unsafe { fcntl(fd, F_GETFL, 0) };
    if flag < 0 {
        return flag;
    }
    flag |= O_NONBLOCK;
    unsafe { fcntl(fd, F_SETFL, flag) }
}

#[inline]
pub fn set_cloexec(fd: c_int) -> c_int {
    unsafe { fcntl(fd, F_SETFD, FD_CLOEXEC) }
}

pub fn check_iso_code(address: IpAddr, iso_code: &str) -> bool {
    let buf = &GEOIP2_COUNTRY_MMDB_BUF;
    let from_source_ret = Reader::from_source(buf.to_vec());
    if from_source_ret.is_err() {
        return false;
    }
    let reader = from_source_ret.unwrap();
    let lookup_ret = reader.lookup::<Country>(address);
    if lookup_ret.is_err() {
        return false;
    }
    let opt_country = lookup_ret.unwrap().country;
    if opt_country.is_none() {
        return false;
    }
    let country = opt_country.unwrap();
    seeval!(country);
    country.iso_code == Some(iso_code)
}

#[inline]
pub fn is_cn_ip(address: IpAddr) -> bool {
    check_iso_code(address, "CN")
}

#[inline]
pub fn what_is_my_ip() -> Option<String> {
    if let Ok(udp_sock) = UdpSocket::bind("0.0.0.0:0") {
        if let Ok(()) = udp_sock.connect("1.1.1.1:53") {
            if let Ok(addr) = udp_sock.local_addr() {
                return Some(addr.ip().to_string());
            }
        }
    }
    None
}

#[macro_export(local_inner_macros)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        std::print!($($arg)*);
    }
}

#[macro_export(local_inner_macros)]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        std::println!($($arg)*)
    }
}

#[macro_export(local_inner_macros)]
macro_rules! seeval {
    ($val:expr) => {
        debug_println!(
            "[{}:{}] {} >>> {:?}",
            core::file!(),
            core::line!(),
            core::stringify!($val),
            $val
        )
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_check_iso_code() {
        let check_iso_code_ret = super::check_iso_code("140.205.135.3".parse().unwrap(), "CN");
        assert_eq!(check_iso_code_ret, true);
        let check_iso_code_ret = super::check_iso_code("172.217.163.46".parse().unwrap(), "US");
        assert_eq!(check_iso_code_ret, true);
    }

    #[test]
    fn test_is_cn_ip() {
        let is_cn_ip_ret = super::is_cn_ip("39.156.66.10".parse().unwrap());
        assert_eq!(is_cn_ip_ret, true);
        let is_cn_ip_ret = super::is_cn_ip("172.217.160.110".parse().unwrap());
        assert_eq!(is_cn_ip_ret, false);
    }
}
