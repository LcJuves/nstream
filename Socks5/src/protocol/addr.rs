use super::AddressType;

use core::mem::size_of;
use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};
use std::str::FromStr;
use std::vec::IntoIter;

use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, Clone, PartialEq)]
pub enum Address {
    IP(SocketAddr),
    Domain(String, u16),
}

impl From<SocketAddrV4> for Address {
    fn from(v4addr: SocketAddrV4) -> Self {
        Self::IP(SocketAddr::V4(v4addr))
    }
}

impl From<SocketAddrV6> for Address {
    fn from(v6addr: SocketAddrV6) -> Self {
        Self::IP(SocketAddr::V6(v6addr))
    }
}

impl From<SocketAddr> for Address {
    fn from(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4addr) => v4addr.into(),
            SocketAddr::V6(v6addr) => v6addr.into(),
        }
    }
}

impl From<(IpAddr, u16)> for Address {
    fn from(pair: (IpAddr, u16)) -> Self {
        Self::IP(SocketAddr::new(pair.0, pair.1))
    }
}

impl From<(Ipv4Addr, u16)> for Address {
    fn from(pair: (Ipv4Addr, u16)) -> Self {
        (IpAddr::V4(pair.0), pair.1).into()
    }
}

impl From<(Ipv6Addr, u16)> for Address {
    fn from(pair: (Ipv6Addr, u16)) -> Self {
        (IpAddr::V6(pair.0), pair.1).into()
    }
}

impl TryFrom<String> for Address {
    type Error = Box<dyn std::error::Error>;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let addr = value;
        let split_idx = addr.rfind(":").unwrap();
        let addr_splitted = addr.split_at(split_idx);
        let ip_addr_or_domain = addr_splitted.0.replace("[", "").replace("]", "");
        let port = u16::from_str(&(addr_splitted.1)[1..])?;
        if let Ok(ip_v4_addr) = ip_addr_or_domain.parse::<Ipv4Addr>() {
            Ok(Self::IP(SocketAddr::V4(SocketAddrV4::new(ip_v4_addr, port))))
        } else if let Ok(ip_v6_addr) = ip_addr_or_domain.parse::<Ipv6Addr>() {
            Ok(Self::IP(SocketAddr::V6(SocketAddrV6::new(ip_v6_addr, port, 0, 0))))
        } else {
            Ok(Self::Domain(ip_addr_or_domain, port))
        }
    }
}

impl ToSocketAddrs for Address {
    type Iter = IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        self.to_string().to_socket_addrs()
    }
}

impl TryInto<Option<SocketAddr>> for Address {
    type Error = std::io::Error;
    fn try_into(self) -> std::result::Result<Option<SocketAddr>, Self::Error> {
        let mut socket_addrs = self.to_socket_addrs()?;
        Ok(socket_addrs.next())
    }
}

impl TryInto<SocketAddr> for Address {
    type Error = std::io::Error;
    fn try_into(self) -> std::result::Result<SocketAddr, Self::Error> {
        let opt_socket_addr = TryInto::<Option<SocketAddr>>::try_into(self)?;
        let socket_addr = opt_socket_addr.ok_or("Invalid address").unwrap();
        Ok(socket_addr)
    }
}

impl ToString for Address {
    fn to_string(&self) -> String {
        match self {
            Self::IP(addr) => {
                let addr_is_ipv6 = addr.is_ipv6();
                let mut ret = String::new();
                if addr_is_ipv6 {
                    ret.push('[');
                }
                ret.push_str(&addr.ip().to_string());
                if addr_is_ipv6 {
                    ret.push(']');
                }
                ret.push_str(&format!(":{}", addr.port()));
                ret
            }
            Self::Domain(name, port) => format!("{}:{}", name, port),
        }
    }
}

impl Address {
    /// Parse [Address] as follows:
    ///
    ///
    /// ```plain
    ///      +------+------+------+
    ///      | ATYP | ADDR | PORT |
    ///      +------+------+------+
    ///      |  1   |  Var |   2  |
    ///      +------+------+------+
    /// ```
    pub(crate) async fn from_socks_bytes<R>(r: &mut R, atyp: &AddressType) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let addr: Address = match atyp {
            AddressType::IPV4 => (
                Ipv4Addr::new(
                    r.read_u8().await?,
                    r.read_u8().await?,
                    r.read_u8().await?,
                    r.read_u8().await?,
                ),
                /* port */ r.read_u16().await?,
            )
                .into(),
            AddressType::FQDN => {
                let dnlen = (r.read_u8().await?) as usize;
                let mut buf = vec![0u8; dnlen];
                r.read(&mut buf).await?;
                Address::Domain(
                    String::from_utf8_lossy(&buf).to_string(),
                    /* port */ r.read_u16().await?,
                )
            }
            AddressType::IPV6 => (
                Ipv6Addr::new(
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                    r.read_u16().await?,
                ),
                /* port */ r.read_u16().await?,
            )
                .into(),
        };

        Ok(addr)
    }

    /// Serialize [Address] as follows:
    ///
    ///
    /// ```plain
    ///      +------+------+
    ///      | ADDR | PORT |
    ///      +------+------+
    ///      |  Var |   2  |
    ///      +------+------+
    /// ```
    pub(crate) fn as_socks_bytes(&self) -> Vec<u8> {
        let mut ret = vec![];
        match self {
            Self::IP(addr) => match addr.ip() {
                IpAddr::V4(v4addr) => ret.extend_from_slice(&v4addr.octets()),
                IpAddr::V6(v6addr) => ret.extend_from_slice(&v6addr.octets()),
            },
            Self::Domain(name, _) => {
                let name_bytes = name.as_bytes();
                ret.push(name_bytes.len() as u8);
                ret.extend_from_slice(name_bytes);
            }
        }
        ret.extend_from_slice(&self.port().to_be_bytes()); /* PORT */
        ret
    }

    pub fn port(&self) -> u16 {
        match self {
            Self::IP(addr) => addr.port(),
            Self::Domain(_, port) => *port,
        }
    }

    pub fn len(&self) -> usize {
        let ip_or_domain_size: usize;
        match self {
            Self::IP(addr) => match addr.ip() {
                IpAddr::V4(_) => ip_or_domain_size = size_of::<u8>() * 4,
                IpAddr::V6(_) => ip_or_domain_size = size_of::<u16>() * 8,
            },
            Self::Domain(name, _) => ip_or_domain_size = name.as_bytes().len(),
        }
        ip_or_domain_size + size_of::<u16>()
    }

    #[inline]
    pub fn binary_len(&self) -> usize {
        self.len() * 8
    }
}

impl Default for Address {
    fn default() -> Self {
        Address::IP(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))
    }
}

#[test]
fn test_to_string() {
    let ipv4_addr: Address = (Ipv4Addr::LOCALHOST, 80).into();
    assert_eq!(ipv4_addr.to_string(), "127.0.0.1:80");

    let ipv6_addr: Address = (Ipv6Addr::LOCALHOST, 80).into();
    assert_eq!(ipv6_addr.to_string(), "[::1]:80");
}

#[test]
fn test_from_socks_bytes() -> Result<()> {
    use tokio::io::BufReader;
    let tokio_rt = tokio::runtime::Runtime::new()?;

    let v4bytes = vec![127, 0, 0, 1, 0x00, 0x50];
    let mut v4bufrd = BufReader::new(&v4bytes[..]);
    let v4addr = tokio_rt.block_on(Address::from_socks_bytes(&mut v4bufrd, &AddressType::IPV4))?;
    assert_eq!(v4addr, (Ipv4Addr::LOCALHOST, 80).into());

    let dnbytes = vec![
        /* dnlen */ 10, /* dnlen */
        103, 105, 116, 104, 117, 98, 46, 99, 111, 109, /* begin port */ 0x01, 0xbb,
    ];
    let mut dnbufrd = BufReader::new(&dnbytes[..]);
    let dnaddr = tokio_rt.block_on(Address::from_socks_bytes(&mut dnbufrd, &AddressType::FQDN))?;
    assert_eq!(dnaddr, Address::Domain(String::from("github.com"), 443));

    let v6bytes = vec![
        0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x02, 0x0c, 0x29, 0xff, 0xfe, 0x96, 0x8b,
        0x55, /* begin port */ 0x1f, 0x90,
    ];
    let mut v6bufrd = BufReader::new(&v6bytes[..]);
    let v6addr = tokio_rt.block_on(Address::from_socks_bytes(&mut v6bufrd, &AddressType::IPV6))?;
    assert_eq!(
        v6addr,
        ("2001:db8:1:0:20c:29ff:fe96:8b55".parse::<Ipv6Addr>().unwrap(), 8080).into()
    );

    Ok(())
}

#[test]
fn test_as_socks_bytes() {
    let ipv4_addr: Address = (Ipv4Addr::LOCALHOST, 80).into();
    assert_eq!(ipv4_addr.as_socks_bytes(), vec![127, 0, 0, 1, 0x00, 0x50]);

    let addr_dn: Address = Address::Domain(String::from("github.com"), 443);
    assert_eq!(
        addr_dn.as_socks_bytes(),
        vec![
            /* dnlen */ 10, /* dnlen */
            103, 105, 116, 104, 117, 98, 46, 99, 111, 109, /* begin port */ 0x01, 0xbb
        ]
    );

    let ipv6_addr: Address =
        (Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x20c, 0x29ff, 0xfe96, 0x8b55), 8080).into();

    assert_eq!(
        ipv6_addr.as_socks_bytes(),
        vec![
            0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x02, 0x0c, 0x29, 0xff, 0xfe, 0x96,
            0x8b, 0x55, /* begin port */ 0x1f, 0x90
        ]
    );
}

#[test]
fn test_len() {
    let addr = Address::default();
    assert_eq!(addr.len(), 6);

    let addr: Address = (Ipv6Addr::LOCALHOST, 4433).into();
    assert_eq!(addr.len(), 18);

    let addr = Address::Domain(String::from("github.com"), 443);
    assert_eq!(addr.len(), 12);
}

#[test]
fn test_binary_len() {
    let addr = Address::default();
    assert_eq!(addr.binary_len(), 48);

    let addr: Address = (Ipv6Addr::LOCALHOST, 4433).into();
    assert_eq!(addr.binary_len(), 144);

    let addr = Address::Domain(String::from("github.com"), 443);
    assert_eq!(addr.binary_len(), 96);
}

#[test]
fn test_try_from() {
    let addr_string = String::from("github.com:443");
    let addr: Address = addr_string.try_into().unwrap();
    assert_eq!(addr, Address::Domain(String::from("github.com"), 443));

    let addr_string = String::from("127.0.0.1:80");
    let addr: Address = addr_string.try_into().unwrap();
    assert_eq!(
        addr,
        Address::IP(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 80)))
    );

    let addr_string = String::from("[::1]:4433");
    let addr: Address = addr_string.try_into().unwrap();
    assert_eq!(
        addr,
        Address::IP(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 4433, 0, 0)))
    );
}

#[test]
fn test_try_into_socket_addr() -> Result<()> {
    let addr = Address::default();
    let socket_addr: SocketAddr = addr.try_into()?;
    assert_eq!(socket_addr.ip(), Ipv4Addr::UNSPECIFIED);
    assert_eq!(socket_addr.port(), 0);
    Ok(())
}
