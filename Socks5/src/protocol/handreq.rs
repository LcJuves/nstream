//! https://www.rfc-editor.org/rfc/rfc1928

use crate::protocol::AuthMethod;

use std::io::Result;

use tokio::io::{AsyncRead, AsyncReadExt};

/// The client connects to the server, and sends a version
/// identifier/method selection message:
///
/// ```plain
///                 +----+----------+----------+
///                 |VER | NMETHODS | METHODS  |
///                 +----+----------+----------+
///                 | 1  |    1     | 1 to 255 |
///                 +----+----------+----------+
/// ```
///
/// The VER field is set to X'05' for this version of the protocol.  The
/// NMETHODS field contains the number of method identifier octets that
/// appear in the METHODS field.
#[derive(Debug, Clone)]
pub struct HandshakeRequest {
    methods: Vec<AuthMethod>,
}

impl HandshakeRequest {
    pub fn new(methods: Vec<AuthMethod>) -> Self {
        Self { methods }
    }

    pub fn methods(&self) -> Vec<AuthMethod> {
        self.methods.to_owned()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![
            /* VER */ crate::SOCKS_VERSION, /* VER */
            /* NMETHODS */ self.methods.len() as u8, /* NMETHODS */
        ];
        for m in self.methods.iter() {
            ret.push((*m).to_owned().into());
        }
        ret
    }
}

impl HandshakeRequest {
    pub async fn from<R>(r: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        if let Err(e) = crate::check_socks_ver(r).await {
            Err(e)
        } else {
            let nmethods = r.read_u8().await? as usize;
            let mut methods = Vec::<AuthMethod>::with_capacity(nmethods);
            for _ in 0..nmethods {
                methods.push(r.read_u8().await?.into());
            }

            Ok(Self { methods })
        }
    }
}
