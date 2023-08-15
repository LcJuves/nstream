//! https://datatracker.ietf.org/doc/html/rfc1928

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Connect,
    Bind,
    UdpAssociate,
}

impl TryFrom<u8> for Command {
    type Error = std::io::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Connect),
            0x02 => Ok(Self::Bind),
            0x03 => Ok(Self::UdpAssociate),
            _ => Err(crate::throw_io_error("Unknown command")),
        }
    }
}

impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Self::Connect => 0x01,
            Self::Bind => 0x02,
            Self::UdpAssociate => 0x03,
        }
    }
}
