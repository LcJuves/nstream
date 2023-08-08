//! https://datatracker.ietf.org/doc/html/rfc1929

use std::io::Result;

use tokio::io::{AsyncRead, AsyncReadExt};

/// Once the SOCKS V5 server has started, and the client has selected the
/// Username/Password Authentication protocol, the Username/Password
/// subnegotiation begins.  This begins with the client producing a
/// Username/Password request:
///
/// ```plain
///         +----+------+----------+------+----------+
///         |VER | ULEN |  UNAME   | PLEN |  PASSWD  |
///         +----+------+----------+------+----------+
///         | 1  |  1   | 1 to 255 |  1   | 1 to 255 |
///         +----+------+----------+------+----------+
/// ```
///
/// The VER field contains the current version of the subnegotiation,
/// which is X'01'. The ULEN field contains the length of the UNAME field
/// that follows. The UNAME field contains the username as known to the
/// source operating system. The PLEN field contains the length of the
/// PASSWD field that follows. The PASSWD field contains the password
/// association with the given UNAME.
#[derive(Debug, Clone)]
pub struct UsernamePasswordAuth {
    usr: String,
    pwd: String,
}

impl UsernamePasswordAuth {
    pub fn new(uname: &str, passwd: &str) -> Self {
        Self { usr: uname.to_string(), pwd: passwd.to_string() }
    }

    pub fn uname(&self) -> String {
        self.usr.to_owned()
    }

    pub fn passwd(&self) -> String {
        self.pwd.to_owned()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![crate::AUTH_VERSION]; /* VER */
        let usr_bytes = self.usr.as_bytes();
        ret.push(usr_bytes.len() as u8); /* ULEN */
        ret.extend_from_slice(&usr_bytes); /* UNAME */
        let pwd_bytes = self.pwd.as_bytes();
        ret.push(pwd_bytes.len() as u8); /* PLEN */
        ret.extend_from_slice(&pwd_bytes); /* PASSWD */
        ret
    }
}

impl UsernamePasswordAuth {
    pub async fn from<R>(r: &mut R) -> Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        if let Err(e) = crate::check_auth_ver(r).await {
            Err(e)
        } else {
            let usrlen = r.read_u8().await? as usize;
            let mut usrbuf = vec![0u8; usrlen];
            r.read_exact(&mut usrbuf).await?;
            let usr = String::from_utf8_lossy(&usrbuf).to_string();

            let pwdlen = r.read_u8().await? as usize;
            let mut pwdbuf = vec![0u8; pwdlen];
            r.read_exact(&mut pwdbuf).await?;
            let pwd = String::from_utf8_lossy(&pwdbuf).to_string();

            Ok(Self { usr, pwd })
        }
    }
}
