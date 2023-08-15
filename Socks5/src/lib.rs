pub mod protocol;

#[cfg(debug_assertions)]
use std::io::Read;
use std::io::{Error, ErrorKind, Result};

use tokio::{
    io::{copy_bidirectional, AsyncRead, AsyncReadExt, AsyncWrite},
    net::TcpStream,
};

pub const SOCKS_VERSION: u8 = 0x05;
pub const AUTH_VERSION: u8 = 0x01;
pub const RSV_RESERVED: u8 = 0x00;

#[inline]
pub(crate) fn throw_io_error(msg: &str) -> Error {
    Error::new(ErrorKind::Unsupported, msg)
}

pub(crate) async fn check_socks_ver<R>(r: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let ver = r.read_u8().await?;
    if ver != SOCKS_VERSION {
        Err(throw_io_error(&format!("Unsupported socks version: {:#04x}", ver)))
    } else {
        Ok(())
    }
}

pub(crate) async fn check_auth_ver<R>(r: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let ver = r.read_u8().await?;
    if ver != AUTH_VERSION {
        Err(throw_io_error(&format!("Unsupported authentication version: {:#04x}", ver)))
    } else {
        Ok(())
    }
}

pub(crate) async fn check_rsv<R>(r: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let rsv = r.read_u8().await?;
    if rsv != RSV_RESERVED {
        Err(throw_io_error(&format!("Unsupported RSV flag: {:#04x}", rsv)))
    } else {
        Ok(())
    }
}

#[inline]
pub async fn exchange_data<F, T>(from: &mut F, to: &mut T) -> Result<(u64, u64)>
where
    F: AsyncRead + AsyncWrite + Unpin + ?Sized,
    T: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    Ok(copy_bidirectional(from, to).await?)
}

pub async fn wait_closed(tcp_stream: &mut TcpStream) -> Result<()> {
    loop {
        match tcp_stream.read(&mut [0]).await {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(err) => break Err(err),
        }
    }
}

#[inline]
#[cfg(debug_assertions)]
#[allow(dead_code)]
pub(crate) fn read_u8_from<R>(r: &mut R) -> Result<u8>
where
    R: Read,
{
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

#[inline]
#[cfg(debug_assertions)]
#[allow(dead_code)]
pub(crate) fn read_u16_from<R>(r: &mut R) -> Result<u16>
where
    R: Read,
{
    let b0 = read_u8_from(r)?;
    let b1 = read_u8_from(r)?;
    Ok(u16::from_be_bytes([b0, b1]))
}

#[cfg(test)]
mod tests {}
