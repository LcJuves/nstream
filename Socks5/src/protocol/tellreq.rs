//! https://www.rfc-editor.org/rfc/rfc1928

use super::Address;
use crate::protocol::AddressType;
use crate::protocol::Command;

use std::io::Result;

use tokio::io::{AsyncRead, AsyncReadExt};

/// Once the method-dependent subnegotiation has completed, the client
/// sends the request details.  If the negotiated method includes
/// encapsulation for purposes of integrity checking and/or
/// confidentiality, these requests MUST be encapsulated in the method-
/// dependent encapsulation.
///
/// The SOCKS request is formed as follows:
///
/// ```plain
///      +----+-----+-------+------+----------+----------+
///      |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
///      +----+-----+-------+------+----------+----------+
///      | 1  |  1  | X'00' |  1   | Variable |    2     |
///      +----+-----+-------+------+----------+----------+
/// ```
#[derive(Debug, Clone)]
pub struct TellRequest {
    cmd: Command,
    atyp: AddressType,
    /// This content format is as follows:
    ///     ```127.0.0.1:80```, ```github.com:443``` or ```[2001:db8:1:0:20c:29ff:fe96:8b55]:8080```
    addr: Address,
}

impl TellRequest {
    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        let mut ret = vec![
            crate::SOCKS_VERSION,        /* VER */
            self.cmd.to_owned().into(),  /* CMD */
            crate::RSV_RESERVED,         /* RSV */
            self.atyp.to_owned().into(), /* ATYP */
        ];
        ret.extend_from_slice(&self.addr.as_socks_bytes());
        Ok(ret)
    }

    pub fn new(cmd: Command, atyp: AddressType, addr: Address) -> Self {
        Self { cmd, atyp, addr }
    }

    pub fn cmd(&self) -> Command {
        self.cmd.to_owned()
    }

    pub fn atyp(&self) -> AddressType {
        self.atyp.to_owned()
    }

    pub fn addr(&self) -> Address {
        self.addr.to_owned()
    }
}

impl TellRequest {
    pub async fn from<R>(r: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        crate::check_socks_ver(r).await?;
        let cmd = r.read_u8().await?.try_into()?;
        crate::check_rsv(r).await?;
        let atyp = r.read_u8().await?.try_into()?;
        let addr = Address::from_socks_bytes(r, &atyp).await?;
        Ok(Self { cmd, atyp, addr })
    }
}

#[test]
fn test_from() -> std::io::Result<()> {
    use tokio::io::BufReader;
    let tokio_rt = tokio::runtime::Runtime::new()?;

    let v4reqbytes = [5u8, 1, 0, 1, 127, 0, 0, 1, 0x00, 0x50];
    let mut v4reqbufrd = BufReader::new(&v4reqbytes[..]);
    let v4req = tokio_rt.block_on(TellRequest::from(&mut v4reqbufrd))?;
    assert!(v4req.cmd == Command::Connect);
    assert!(v4req.atyp == AddressType::IPV4);
    assert_eq!(v4req.addr, (std::net::Ipv4Addr::LOCALHOST, 80).into());

    let dnreqbytes = [
        5u8, 1, 0, 3, /* dnlen */ 10, /* dnlen */
        103, 105, 116, 104, 117, 98, 46, 99, 111, 109, /* begin port */ 0x01, 0xbb,
    ];
    let mut dnreqbufrd = BufReader::new(&dnreqbytes[..]);
    let dnreq = tokio_rt.block_on(TellRequest::from(&mut dnreqbufrd))?;
    assert!(dnreq.atyp == AddressType::FQDN);
    assert_eq!(dnreq.addr, Address::Domain(String::from("github.com"), 443));

    let v6reqbytes = [
        5u8, 1, 0, 4, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x02, 0x0c, 0x29, 0xff, 0xfe,
        0x96, 0x8b, 0x55, /* begin port */ 0x1f, 0x90,
    ];
    let mut v6reqbufrd = BufReader::new(&v6reqbytes[..]);
    let v6req = tokio_rt.block_on(TellRequest::from(&mut v6reqbufrd))?;
    assert!(v6req.atyp == AddressType::IPV6);
    assert_eq!(
        v6req.addr,
        ("2001:db8:1:0:20c:29ff:fe96:8b55".parse::<std::net::Ipv6Addr>().unwrap(), 8080).into()
    );

    Ok(())
}
