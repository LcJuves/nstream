//! https://datatracker.ietf.org/doc/html/rfc1928

use crate::protocol::AddressType;

use super::Address;

use std::net::{IpAddr, SocketAddr};

use tokio::io::{BufReader, Result};
use tokio::net::UdpSocket;

/// A UDP-based client MUST send its datagrams to the UDP relay server at
/// the UDP port indicated by BND.PORT in the reply to the UDP ASSOCIATE
/// request.  If the selected authentication method provides
/// encapsulation for the purposes of authenticity, integrity, and/or
/// confidentiality, the datagram MUST be encapsulated using the
/// appropriate encapsulation.  Each UDP datagram carries a UDP request
/// header with it:
///
/// ```plain
///      +----+------+------+----------+----------+----------+
///      |RSV | FRAG | ATYP | DST.ADDR | DST.PORT |   DATA   |
///      +----+------+------+----------+----------+----------+
///      | 2  |  1   |  1   | Variable |    2     | Variable |
///      +----+------+------+----------+----------+----------+
/// ```
#[derive(Debug, Clone)]
pub struct UdpPacket {
    /// Current fragment number
    frag: u8,
    /// This content format is as follows:
    ///     ```127.0.0.1:80```, ```github.com:443``` or ```[2001:db8:1:0:20c:29ff:fe96:8b55]:8080```
    addr: Address,
    /// User data
    data: Vec<u8>,
}

impl UdpPacket {
    #[inline]
    pub fn new(frag: u8, addr: Address, data: Vec<u8>) -> Self {
        Self { frag, addr, data }
    }

    pub async fn from(udp_sock: &UdpSocket) -> Result<(Self, SocketAddr)> {
        loop {
            // The buffer is **not** included in the async task and will only exist
            // on the stack.
            let mut udp_data = [0u8; u16::MAX as usize];
            let (len, from_addr) = udp_sock.recv_from(&mut udp_data).await?;
            let udp_data = &udp_data[..len];
            if len <= 4 {
                return Err(crate::throw_io_error(&format!(
                    "Readied unknown data: {:?}",
                    udp_data
                )));
            }
            let _rsv = u16::from_be_bytes([udp_data[0], udp_data[1]]); /* TODO: Check it */
            let frag = udp_data[2];
            let atyp: AddressType = udp_data[3].try_into()?;
            let mut addr_buf = BufReader::new(&udp_data[4..]);
            if let Ok(to_addr) = Address::from_socks_bytes(&mut addr_buf, &atyp).await {
                let data = (&udp_data[(4 + to_addr.len())..]).to_vec();
                return Ok((Self::new(frag, to_addr, data), from_addr));
            }

            // Err(crate::throw_io_error(&format!("Readied unknown data: {:?}", udp_data)))
        }
    }

    #[inline]
    pub fn frag(&self) -> u8 {
        self.frag.to_owned()
    }

    #[inline]
    pub fn addr(&self) -> Address {
        self.addr.to_owned()
    }

    #[inline]
    pub fn data(&self) -> Vec<u8> {
        self.data.to_owned()
    }

    pub fn as_socks_bytes(&self) -> Vec<u8> {
        let mut ret = vec![];
        ret.extend_from_slice(&[0x00, 0x00]); /* RSV */
        ret.push(self.frag()); /* FRAG */
        let addr = self.addr();
        let addr_bytes = addr.as_socks_bytes();
        let atyp = Into::<AddressType>::into(addr);
        ret.push(atyp.into()); /* ATYP */
        ret.extend_from_slice(&addr_bytes); /* DST.ADDR DST.PORT */
        ret.extend_from_slice(&self.data()); /* DATA */
        ret
    }

    pub async fn new_exchange(listen_ip: IpAddr) -> Result<(UdpSocket, UdpSocket)> {
        let zero_addr = SocketAddr::from(([0, 0, 0, 0], 0));
        let from_socket_addr = SocketAddr::from((listen_ip, 0u16));
        let from_udp_sock = UdpSocket::bind(from_socket_addr).await?;
        let to_udp_sock = UdpSocket::bind(zero_addr).await?;
        Ok((from_udp_sock, to_udp_sock))
    }
}

#[test]
fn test_as_socks_bytes() {
    let data = vec![
        219u8, 221, 1, 32, 0, 1, 0, 0, 0, 0, 0, 0, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 0,
        28, 0, 1,
    ];
    let udp_pack = UdpPacket::new(0, Address::default(), data);
    let udp_pack_bytes = udp_pack.as_socks_bytes();
    assert_eq!(
        udp_pack_bytes,
        vec![
            0u8, 0, 0, 1, 0, 0, 0, 0, /* PROT */ 0u8, 0u8, /* PORT */
            /* DATA */ 219, 221, 1, 32, 0, 1, 0, 0, 0, 0, 0, 0, 5, 98, 97, 105, 100, 117, 3,
            99, 111, 109, 0, 0, 28, 0, 1,
        ]
    )
}
