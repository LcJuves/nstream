//! https://datatracker.ietf.org/doc/html/rfc1928

use super::Address;

use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq)]
pub enum AddressType {
    // the address is a version-4 IP address, with a length of 4 octets.
    IPV4,

    // the address field contains a fully-qualified domain name.  The first
    // octet of the address field contains the number of octets of name that
    // follow, there is no terminating NUL octet.
    FQDN,

    // the address is a version-6 IP address, with a length of 16 octets.
    IPV6,
}

impl TryFrom<u8> for AddressType {
    type Error = std::io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::IPV4),
            0x03 => Ok(Self::FQDN),
            0x04 => Ok(Self::IPV6),
            _ => Err(crate::throw_io_error("Unknown address type")),
        }
    }
}

impl Into<u8> for AddressType {
    fn into(self) -> u8 {
        match self {
            Self::IPV4 => 0x01,
            Self::FQDN => 0x03,
            Self::IPV6 => 0x04,
        }
    }
}

impl Default for AddressType {
    fn default() -> Self {
        AddressType::IPV4
    }
}

impl From<Address> for AddressType {
    fn from(value: Address) -> Self {
        match value {
            Address::IP(addr) => match addr.ip() {
                IpAddr::V4(_) => Self::IPV4,
                IpAddr::V6(_) => Self::IPV6,
            },
            Address::Domain(_, _) => Self::FQDN,
        }
    }
}

#[test]
fn test_from_address() {
    let addr = Address::default();
    assert_eq!(Into::<AddressType>::into(addr), AddressType::IPV4);

    let addr: Address = (std::net::Ipv6Addr::LOCALHOST, 0).into();
    assert_eq!(Into::<AddressType>::into(addr), AddressType::IPV6);

    let addr = Address::Domain("github.com".to_string(), 443);
    assert_eq!(Into::<AddressType>::into(addr), AddressType::FQDN);
}
