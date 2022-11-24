//! https://www.rfc-editor.org/rfc/rfc1928

use crate::protocol::AuthMethod;

use std::io::Result;

use tokio::io::{AsyncRead, AsyncReadExt};

/// The server selects from one of the methods given in METHODS, and
/// sends a METHOD selection message:
///
/// ```plain
///                       +----+--------+
///                       |VER | METHOD |
///                       +----+--------+
///                       | 1  |   1    |
///                       +----+--------+
/// ```
///
/// If the selected METHOD is X'FF', none of the methods listed by the
/// client are acceptable, and the client MUST close the connection.
#[derive(Debug, Clone)]
pub struct HandshakeResponse {
    method: AuthMethod,
}

impl HandshakeResponse {
    pub fn new(method: AuthMethod) -> Self {
        Self { method }
    }

    pub fn method(&self) -> AuthMethod {
        self.method.to_owned()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        vec![crate::SOCKS_VERSION, self.method.to_owned().into()]
    }
}

impl HandshakeResponse {
    pub async fn from<R>(r: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        if let Err(e) = crate::check_socks_ver(r).await {
            Err(e)
        } else {
            Ok(Self { method: (r.read_u8().await?).into() })
        }
    }
}
