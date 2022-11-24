mod cmd;

use std::io::ErrorKind;
use std::sync::Arc;

use socks5::exchange_data;
use socks5::protocol::{
    AuthMethod, HandshakeRequest, HandshakeResponse, ReplyField, ReplyResponse, TellRequest,
};

use tokio::io::{copy, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let usr = Arc::new("aaa".to_string());
    let pwd = Arc::new("bbb".to_string());

    crate::cmd::close_socks5_proxy()?;
    crate::cmd::open_socks5_proxy(usr.as_str(), pwd.as_str())?;
    let listener = TcpListener::bind(format!(
        "{}:{}",
        crate::cmd::SOCKS5_PROXY_HOST_IP_ADDR,
        crate::cmd::SOCKS5_PROXY_HOST_PORT
    ))
    .await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        let _usr = usr.clone();
        let _pwd = pwd.clone();

        tokio::spawn(async move {
            let hreq = HandshakeRequest::from(&mut socket).await?;
            if hreq.methods().contains(&AuthMethod::NoAuthenticationRequired) {
                // println!("{:?}", hreq);
            }
            let hresp = HandshakeResponse::new(AuthMethod::NoAuthenticationRequired);
            if let Err(e) = &mut socket.write(&hresp.as_bytes()).await {
                eprintln!("Failed to write handshake response; error: {:?}", e);
            }

            let tellreq = TellRequest::from(&mut socket).await.unwrap();
            tokio::spawn(async move {
                let tellret = TcpStream::connect(tellreq.addr().to_string()).await;
                let rep = match tellret {
                    Ok(_) => ReplyField::Succeeded,
                    Err(ref e) => match e.kind() {
                        ErrorKind::ConnectionRefused => ReplyField::ConnectionRefused,
                        // ErrorKind::NetworkDown |
                        ErrorKind::ConnectionReset | ErrorKind::NotConnected => {
                            ReplyField::GeneralSocksServerFailure
                        }
                        // ErrorKind::HostUnreachable => ReplyField::HostUnreachable,
                        // ErrorKind::NetworkUnreachable => ReplyField::NetworkUnreachable,
                        ErrorKind::ConnectionAborted => ReplyField::ConnectionNotAllowedByRuleSet,
                        ErrorKind::TimedOut => ReplyField::TTLExpired,
                        ErrorKind::Other | _ => ReplyField::Unassigned,
                    },
                };

                let represp = ReplyResponse::new(rep, tellreq.atyp(), tellreq.addr());
                let resp_bytes = represp.as_bytes().unwrap();
                let mut represp_bytes_buf = BufReader::new(&resp_bytes[..]);
                copy(&mut represp_bytes_buf, &mut socket).await?;

                if represp.rep() == ReplyField::Succeeded {
                    let mut tell_stream = tellret?;
                    tokio::spawn(async move {
                        exchange_data(&mut tell_stream, &mut socket).await?;
                        tell_stream.shutdown().await?;
                        socket.shutdown().await
                    });
                }

                Ok::<_, std::io::Error>(())
            });

            Ok::<_, std::io::Error>(())
        });
    }
}
