//! ```sh
//! networksetup -setwebproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! networksetup -setsecurewebproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! networksetup -setsocksfirewallproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! ```
//! </br>
//!
//! ```sh
//! alias proxy='export all_proxy="socks5://$proxy_host_ip_addr:$proxy_host_port" && \
//! networksetup -setwebproxystate $networkservice on && \
//! networksetup -setsecurewebproxystate $networkservice on && \
//! networksetup -setsocksfirewallproxystate $networkservice on'
//! ```

#[cfg(target_os = "macos")]
use std::ffi::OsStr;
use std::io::Result;
#[cfg(target_os = "macos")]
use std::process::{Command, ExitStatus, Stdio};

pub(crate) const SOCKS5_PROXY_HOST_PORT: u16 = 19934;

#[cfg(target_os = "macos")]
pub(crate) const NETWORK_SERVICE: &'static str = "Wi-Fi";

#[cfg(target_os = "macos")]
#[inline]
fn exec_networksetup<S: AsRef<OsStr>>(args: &[S]) -> Result<ExitStatus> {
    let mut cmd_networksetup = Command::new("networksetup");
    let mut cmd_networksetup = cmd_networksetup.stdout(Stdio::null()).stderr(Stdio::null());
    for arg in args {
        cmd_networksetup = cmd_networksetup.arg(arg);
    }
    cmd_networksetup.status()
}

#[cfg(target_os = "macos")]
#[allow(unused_variables)]
#[allow(dead_code)]
pub(crate) fn open_socks5_proxy(ip: &str, usr: &str, pwd: &str) -> Result<()> {
    assert!(exec_networksetup(&[
        "-setsocksfirewallproxy",
        NETWORK_SERVICE,
        ip,
        &SOCKS5_PROXY_HOST_PORT.to_string()
    ])?
    .success());

    assert!(exec_networksetup(&["-setwebproxystate", NETWORK_SERVICE, "off"])?.success());
    assert!(exec_networksetup(&["-setsecurewebproxystate", NETWORK_SERVICE, "off"])?.success());
    assert!(exec_networksetup(&["-setsocksfirewallproxystate", NETWORK_SERVICE, "on"])?.success());
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn close_socks5_proxy() -> Result<()> {
    assert!(exec_networksetup(&["-setsocksfirewallproxystate", NETWORK_SERVICE, "off"])?.success());
    Ok(())
}
