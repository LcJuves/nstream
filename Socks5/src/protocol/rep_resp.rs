//! https://datatracker.ietf.org/doc/html/rfc1928

use super::{Address, AddressType, ReplyField};

use tokio::io::{copy, AsyncRead, AsyncReadExt, AsyncWrite, BufReader, Result};

/// The SOCKS request information is sent by the client as soon as it has
/// established a connection to the SOCKS server, and completed the
/// authentication negotiations.  The server evaluates the request, and
/// returns a reply formed as follows:
///
/// ```plain
///      +----+-----+-------+------+----------+----------+
///      |VER | REP |  RSV  | ATYP | BND.ADDR | BND.PORT |
///      +----+-----+-------+------+----------+----------+
///      | 1  |  1  | X'00' |  1   | Variable |    2     |
///      +----+-----+-------+------+----------+----------+
/// ```
#[derive(Debug, Clone)]
pub struct ReplyResponse {
    rep: ReplyField,
    /// This content format is as follows:
    ///     ```127.0.0.1:80```, ```github.com:443``` or ```[2001:db8:1:0:20c:29ff:fe96:8b55]:8080```
    addr: Address,
}

impl ReplyResponse {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![
            crate::SOCKS_VERSION, /* VER */
            self.rep().into(),    /* REP */
            crate::RSV_RESERVED,  /* RSV */
            self.atyp().into(),   /* ATYP */
        ];
        ret.extend_from_slice(&self.addr.as_socks_bytes());
        ret
    }

    #[inline]
    pub fn new(rep: ReplyField, addr: Address) -> Self {
        Self { rep, addr }
    }

    #[inline]
    pub fn rep(&self) -> ReplyField {
        self.rep.to_owned()
    }

    #[inline]
    pub fn atyp(&self) -> AddressType {
        self.addr().into()
    }

    #[inline]
    pub fn addr(&self) -> Address {
        self.addr.to_owned()
    }

    pub async fn respond_with<'a, W>(&self, writer: &'a mut W) -> Result<u64>
    where
        W: AsyncWrite + Unpin + ?Sized,
    {
        let resp_bytes = self.as_bytes();
        let mut resp_bytes_reader = BufReader::new(resp_bytes.as_slice());
        copy(&mut resp_bytes_reader, writer).await
    }
}

impl ReplyResponse {
    pub async fn from<R>(r: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        crate::check_socks_ver(r).await?;
        let rep = r.read_u8().await?.into();
        crate::check_rsv(r).await?;
        let atyp = r.read_u8().await?.try_into()?;
        let addr = Address::from_socks_bytes(r, &atyp).await?;
        Ok(Self { rep, addr })
    }
}

#[test]
fn test_from() -> std::io::Result<()> {
    let tokio_rt = tokio::runtime::Runtime::new()?;

    let v4reqbytes = [5u8, 1, 0, 1, 127, 0, 0, 1, 0x00, 0x50];
    let mut v4reqbufrd = BufReader::new(&v4reqbytes[..]);
    let v4req = tokio_rt.block_on(ReplyResponse::from(&mut v4reqbufrd))?;
    assert!(v4req.rep == ReplyField::GeneralSocksServerFailure);
    assert!(v4req.atyp() == AddressType::IPV4);
    assert_eq!(v4req.addr, (std::net::Ipv4Addr::LOCALHOST, 80).into());

    let dnreqbytes = [
        5u8, 1, 0, 3, /* dnlen */ 10, /* dnlen */
        103, 105, 116, 104, 117, 98, 46, 99, 111, 109, /* begin port */ 0x01, 0xbb,
    ];
    let mut dnreqbufrd = BufReader::new(&dnreqbytes[..]);
    let dnreq = tokio_rt.block_on(ReplyResponse::from(&mut dnreqbufrd))?;
    assert!(dnreq.atyp() == AddressType::FQDN);
    assert_eq!(dnreq.addr, Address::Domain(String::from("github.com"), 443));

    let v6reqbytes = [
        5u8, 1, 0, 4, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x02, 0x0c, 0x29, 0xff, 0xfe,
        0x96, 0x8b, 0x55, /* begin port */ 0x1f, 0x90,
    ];
    let mut v6reqbufrd = BufReader::new(&v6reqbytes[..]);
    let v6req = tokio_rt.block_on(ReplyResponse::from(&mut v6reqbufrd))?;
    assert!(v6req.atyp() == AddressType::IPV6);
    assert_eq!(
        v6req.addr,
        ("2001:db8:1:0:20c:29ff:fe96:8b55".parse::<std::net::Ipv6Addr>().unwrap(), 8080).into()
    );

    Ok(())
}
