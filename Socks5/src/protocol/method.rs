//! https://www.rfc-editor.org/rfc/rfc1928

/// The values currently defined for METHOD are:
///
/// - `0x00` NO AUTHENTICATION REQUIRED
/// - `0x01` GSSAPI
/// - `0x02` USERNAME/PASSWORD
/// - `0x03` to `0x7F` IANA ASSIGNED
/// - `0x80` to `0xFE` RESERVED FOR PRIVATE METHODS
/// - `0xFF` NO ACCEPTABLE METHODS
#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    NoAuthenticationRequired,
    GSSApi,
    UsernameOrPassword,
    IANAAssigned,
    ReservedForPrivateMethods,
    NoAcceptableMethods,
}

impl From<u8> for AuthMethod {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::NoAuthenticationRequired,
            0x01 => Self::GSSApi,
            0x02 => Self::UsernameOrPassword,
            0x03..=0x7f => Self::IANAAssigned,
            0x80..=0xfe => Self::ReservedForPrivateMethods,
            0xff => Self::NoAcceptableMethods,
        }
    }
}

impl Into<u8> for AuthMethod {
    fn into(self) -> u8 {
        match self {
            Self::NoAuthenticationRequired => 0x00,
            Self::GSSApi => 0x01,
            Self::UsernameOrPassword => 0x02,
            Self::IANAAssigned => 0x03,
            Self::ReservedForPrivateMethods => 0x80,
            Self::NoAcceptableMethods => 0xff,
        }
    }
}
