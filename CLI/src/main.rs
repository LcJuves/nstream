mod cmd;

use core::net::{Ipv6Addr, SocketAddr};
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddrV6};
use std::sync::Arc;

use advanced_random_string::{charset, random_string};
use socks5::protocol::{
    Address, AuthMethod, Command, HandshakeRequest, HandshakeResponse, ReplyField, ReplyResponse,
    TellRequest, UdpPacket,
};
use socks5::{exchange_data, wait_closed};

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::sync::Mutex;

use nstream_core::{
    seeval, what_is_my_extip_v4addr, what_is_my_extip_v6addr, what_is_my_lanip_v4addr,
    what_is_my_lanip_v6addr, Tun, VTun, VTunConfig,
};

async fn register_graceful_shutdown() {
    let close_socks5_proxy_and_exit = || {
        crate::cmd::close_socks5_proxy().unwrap();
        std::process::exit(0)
    };
    match signal::ctrl_c().await {
        Ok(()) => {
            println!(" (Received Ctrl + C)");
            close_socks5_proxy_and_exit()
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
            close_socks5_proxy_and_exit()
        }
    }
}

async fn impl_connect(
    tellreq_addr: &SocketAddr,
    tcp_stream: &mut TcpStream,
) -> std::io::Result<()> {
    let proxy_tcp_stream_ret = TcpStream::connect(tellreq_addr).await;
    let rep: ReplyField = (&proxy_tcp_stream_ret).into();
    let rep_resp = ReplyResponse::new(rep, Address::default());
    rep_resp.respond_with(tcp_stream).await?;
    if rep_resp.rep() == ReplyField::Succeeded {
        let mut proxy_tcp_stream = proxy_tcp_stream_ret.unwrap();
        exchange_data(&mut proxy_tcp_stream, tcp_stream).await?;
    } else {
        drop(proxy_tcp_stream_ret);
    }
    tcp_stream.shutdown().await?;
    Ok(())
}

async fn impl_udp_associate(
    tellreq_addr: &SocketAddr,
    tcp_stream: &mut TcpStream,
) -> std::io::Result<()> {
    let listen_ip = tcp_stream.local_addr()?.ip();
    let (from_udp_sock, to_udp_sock) = UdpPacket::new_exchange(listen_ip).await?;
    let connect_ret = (&to_udp_sock).connect(tellreq_addr).await;
    let rep: ReplyField = (&connect_ret).into();

    let rep_resp = ReplyResponse::new(rep, from_udp_sock.local_addr()?.into());
    rep_resp.respond_with(tcp_stream).await?;

    let mut udp_associate_ret = Ok(());
    let incoming_addr = Arc::new(Mutex::new(from_udp_sock.local_addr()?));

    if rep_resp.rep() == ReplyField::Succeeded {
        let _ret = loop {
            tokio::select! {
                _ret = async {
                    let (udp_req, from_addr) = UdpPacket::from(&from_udp_sock).await?;
                    *incoming_addr.lock().await = from_addr;

                    let send_data = udp_req.data();
                    seeval!(&send_data);
                    println!("String(send_data) >>> {}", String::from_utf8_lossy(&send_data));
                    (&to_udp_sock).send(&send_data).await?;
                    Ok::<_, std::io::Error>(())
                } => {
                    if _ret.is_err() {
                        break _ret;
                    }
                },
                _ret = async {
                    let mut back_data = [0u8; u16::MAX as usize];
                    let len = (&to_udp_sock).recv(&mut back_data).await?;
                    let back_data = &back_data[..len];
                    seeval!(back_data);
                    println!("String(back_data) >>> {}", String::from_utf8_lossy(back_data));

                    let from_addr = *incoming_addr.lock().await;

                    let udp_resp = UdpPacket::new(0, tellreq_addr.clone().into(), back_data.to_vec());
                    let udp_resp_bytes = udp_resp.as_socks_bytes();
                    seeval!(udp_resp_bytes);
                    println!();

                    from_udp_sock.send_to(&udp_resp_bytes, from_addr).await?;
                    Ok::<_, std::io::Error>(())
                }  => {
                    if _ret.is_err() {
                        break _ret;
                    }
                },
                _ = wait_closed(tcp_stream) => {
                    break Ok::<_, std::io::Error>(())
                }
            };
        };
        if let err @ Err(_) = _ret {
            udp_associate_ret = err
        }
    }

    tcp_stream.shutdown().await?;
    udp_associate_ret
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tokio::spawn(async { register_graceful_shutdown().await });

    let usr = Arc::new(random_string::generate(10, charset::BASE62));
    let pwd = Arc::new(random_string::generate(10, charset::BASE62));

    let my_extip_v6addr = what_is_my_extip_v6addr().await?;
    seeval!(my_extip_v6addr);
    let my_extip_v4addr = what_is_my_extip_v4addr().await?;
    seeval!(my_extip_v4addr);

    let my_lanip_v6addr =
        what_is_my_lanip_v6addr().await.unwrap_or(Ipv6Addr::LOCALHOST.to_string());
    seeval!(my_lanip_v6addr);
    let my_lanip_v4addr =
        what_is_my_lanip_v4addr().await.unwrap_or(Ipv4Addr::LOCALHOST.to_string());
    seeval!(my_lanip_v4addr);

    crate::cmd::close_socks5_proxy()?;

    let socks5_proxy_bind_addr =
        SocketAddr::V6(SocketAddrV6::new((&my_lanip_v6addr).parse::<Ipv6Addr>().unwrap(), 0, 0, 0));
    let tcp_listener = TcpListener::bind(socks5_proxy_bind_addr).await?;
    let socks5_proxy_bind_addr = tcp_listener.local_addr()?;
    crate::cmd::open_socks5_proxy(socks5_proxy_bind_addr, &usr, &pwd)?;
    let vtun = VTun::new();
    let vtun_config = VTunConfig {
        mtu: Some(2000),
        ipv4_addr: Some(Ipv4Addr::new(192, 168, 31, u8::MAX - 1)),
        ipv6_addr: Some(format!("::ffff:192.168.31.{}", u8::MAX - 1).parse::<Ipv6Addr>().unwrap()),
        netmask: Some(0xffffff00),
    };
    vtun.config_with(vtun_config)?;
    seeval!(vtun.ifname());
    seeval!(vtun.ifindex());
    seeval!(vtun.mtu());

    while let Ok((mut tcp_stream, _)) = tcp_listener.accept().await {
        let _usr = usr.clone();
        let _pwd = pwd.clone();

        tokio::spawn(async move {
            let hreq = HandshakeRequest::from(&mut tcp_stream).await?;
            seeval!(&hreq);
            if hreq.methods().contains(&AuthMethod::NoAuthenticationRequired) {
                // seeval!(hreq);
            }
            let hresp = HandshakeResponse::new(AuthMethod::NoAuthenticationRequired);
            seeval!(&hresp);
            if let Err(e) = (&mut tcp_stream).write(&hresp.as_bytes()).await {
                eprintln!("Failed to write handshake response; error: {:?}", e);
            }

            let tellreq = TellRequest::from(&mut tcp_stream).await?;
            seeval!(&tellreq);
            let tellreq_addr = TryInto::<SocketAddr>::try_into(tellreq.addr())?;
            seeval!(&tellreq_addr);

            seeval!(&tcp_stream);

            match tellreq.cmd() {
                Command::Connect => {
                    tokio::spawn(async move { impl_connect(&tellreq_addr, &mut tcp_stream).await });
                }
                Command::UdpAssociate => {
                    tokio::spawn(async move {
                        impl_udp_associate(&tellreq_addr, &mut tcp_stream).await
                    });
                }
                Command::Bind => {
                    tokio::spawn(async move {
                        let rep_resp =
                            ReplyResponse::new(ReplyField::CommandNotSupported, Address::default());
                        rep_resp.respond_with(&mut tcp_stream).await?;
                        tcp_stream.shutdown().await?;
                        Ok::<_, std::io::Error>(())
                    });
                }
            }
            Ok::<_, std::io::Error>(())
        });
    }

    Ok(())
}
