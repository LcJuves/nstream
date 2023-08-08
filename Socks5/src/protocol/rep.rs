//! https://datatracker.ietf.org/doc/html/rfc1928

#[derive(Debug, Clone, PartialEq)]
pub enum ReplyField {
    Succeeded,
    GeneralSocksServerFailure,
    ConnectionNotAllowedByRuleSet,
    NetworkUnreachable,
    HostUnreachable,
    ConnectionRefused,
    TTLExpired,
    CommandNotSupported,
    AddressTypeNotSupported,
    Unassigned,
}

impl From<u8> for ReplyField {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Succeeded,
            0x01 => Self::GeneralSocksServerFailure,
            0x02 => Self::ConnectionNotAllowedByRuleSet,
            0x03 => Self::NetworkUnreachable,
            0x04 => Self::HostUnreachable,
            0x05 => Self::ConnectionRefused,
            0x06 => Self::TTLExpired,
            0x07 => Self::CommandNotSupported,
            0x08 => Self::AddressTypeNotSupported,
            0x09..=0xff => Self::Unassigned,
        }
    }
}

impl Into<u8> for ReplyField {
    fn into(self) -> u8 {
        match self {
            Self::Succeeded => 0x00,
            Self::GeneralSocksServerFailure => 0x01,
            Self::ConnectionNotAllowedByRuleSet => 0x02,
            Self::NetworkUnreachable => 0x03,
            Self::HostUnreachable => 0x04,
            Self::ConnectionRefused => 0x05,
            Self::TTLExpired => 0x06,
            Self::CommandNotSupported => 0x07,
            Self::AddressTypeNotSupported => 0x08,
            Self::Unassigned => 0x09,
        }
    }
}
