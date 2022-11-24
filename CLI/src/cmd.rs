//! ```sh
//! networksetup -setwebproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! networksetup -setsecurewebproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! networksetup -setsocksfirewallproxy $networkservice $proxy_host_ip_addr $proxy_host_port
//! ```
//! </br>
//!
//! ```sh
//! alias proxy='export http_proxy="${http_protocol_proxy_url}" && \
//! export https_proxy="${http_protocol_proxy_url}" && \
//! export all_proxy="socks5://$proxy_host_ip_addr:$proxy_host_port" && \
//! setup_proxy && \
//! networksetup -setwebproxystate $networkservice on && \
//! networksetup -setsecurewebproxystate $networkservice on && \
//! networksetup -setsocksfirewallproxystate $networkservice on && \
//! sudo dscacheutil -flushcache && \
//! sudo killall -HUP mDNSResponder'
//! ```

use std::io::Result;
use std::process::{Command, Stdio};

pub(crate) const SOCKS5_PROXY_HOST_IP_ADDR: &'static str = "127.0.0.1";
pub(crate) const SOCKS5_PROXY_HOST_PORT: &'static str = "19934";

#[cfg(target_os = "macos")]
pub(crate) const NETWORK_SERVICE: &'static str = "Wi-Fi";

#[cfg(target_os = "macos")]
#[allow(unused_variables)]
pub(crate) fn open_socks5_proxy(usr: &str, pwd: &str) -> Result<()> {
    assert!((Command::new("networksetup")
        .arg("-setsocksfirewallproxy")
        .arg(NETWORK_SERVICE)
        .arg(SOCKS5_PROXY_HOST_IP_ADDR)
        .arg(SOCKS5_PROXY_HOST_PORT)
        // .arg("off")
        // .arg(usr)
        // .arg(pwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?)
    .success());

    assert!((Command::new("networksetup")
        .arg("-setsocksfirewallproxystate")
        .arg(NETWORK_SERVICE)
        .arg("on")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?)
    .success());

    assert!((Command::new("networksetup")
        .arg("-setwebproxystate")
        .arg(NETWORK_SERVICE)
        .arg("off")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?)
    .success());

    assert!((Command::new("networksetup")
        .arg("-setsecurewebproxystate")
        .arg(NETWORK_SERVICE)
        .arg("off")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?)
    .success());

    #[cfg(not(debug_assertions))]
    assert!((Command::new("sudo").arg("dscacheutil").arg("-flushcache").status()?).success());
    #[cfg(not(debug_assertions))]
    assert!(
        (Command::new("sudo").arg("killall").arg("-HUP").arg("mDNSResponder").status()?).success()
    );
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn close_socks5_proxy() -> Result<()> {
    assert!((Command::new("networksetup")
        .arg("-setsocksfirewallproxystate")
        .arg(NETWORK_SERVICE)
        .arg("off")
        .status()?)
    .success());

    #[cfg(not(debug_assertions))]
    assert!((Command::new("sudo").arg("dscacheutil").arg("-flushcache").status()?).success());
    #[cfg(not(debug_assertions))]
    assert!(
        (Command::new("sudo").arg("killall").arg("-HUP").arg("mDNSResponder").status()?).success()
    );
    Ok(())
}
