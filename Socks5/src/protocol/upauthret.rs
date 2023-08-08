//! https://datatracker.ietf.org/doc/html/rfc1929

use std::error::Error;

use tokio::io::{AsyncRead, AsyncReadExt};

/// The server verifies the supplied UNAME and PASSWD, and sends the
/// following response:
///
/// ```plain
///                      +----+--------+
///                      |VER | STATUS |
///                      +----+--------+
///                      | 1  |   1    |
///                      +----+--------+
/// ```
///
/// A STATUS field of X'00' indicates success. If the server returns a
/// `failure' (STATUS value other than X'00') status, it MUST close the
/// connection.
#[derive(Debug, Clone, PartialEq)]
pub enum UsernamePasswordAuthResult {
    Succeeded,
    Failure,
}

impl From<u8> for UsernamePasswordAuthResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Succeeded,
            0x01..=0xff => Self::Failure,
        }
    }
}

impl Into<u8> for UsernamePasswordAuthResult {
    fn into(self) -> u8 {
        match self {
            Self::Succeeded => 0x00,
            Self::Failure => 0x01,
        }
    }
}

impl UsernamePasswordAuthResult {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![crate::AUTH_VERSION]; /* VER */
        ret.push(self.to_owned().into()); /* STATUS */
        ret
    }
}

impl UsernamePasswordAuthResult {
    pub async fn from<R>(r: &mut R) -> Result<Self, Box<dyn Error>>
    where
        R: AsyncRead + Unpin,
    {
        if let Err(e) = crate::check_auth_ver(r).await {
            Err(Box::new(e))
        } else {
            Ok(r.read_u8().await?.try_into()?) /* STATUS */
        }
    }
}

#[test]
fn test_as_bytes() {
    assert_eq!(UsernamePasswordAuthResult::Succeeded.as_bytes(), vec![crate::AUTH_VERSION, 0]);
    assert_eq!(UsernamePasswordAuthResult::Failure.as_bytes(), vec![crate::AUTH_VERSION, 1]);
}
