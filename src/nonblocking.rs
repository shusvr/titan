use crate::{platform as sys, Error, Mode};
use std::{
    io,
    net::IpAddr,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tokio::io::{unix::AsyncFd, AsyncRead, AsyncWrite, ReadBuf};

mod half;
pub use half::*;

/// Async Tun/Tap interface
pub struct Interface {
    inner: AsyncFd<sys::Inner>,
}

impl Interface {
    /// Returns the name of the interface.
    pub fn name(&self) -> &str {
        self.inner.get_ref().name()
    }

    /// Brings the interface up.
    pub fn up(&self) -> io::Result<()> {
        self.inner.get_ref().up()
    }

    /// Assigns ip address to this interface.
    pub fn assign(&self, ip: impl Into<IpAddr>, prefix_len: u32) -> io::Result<()> {
        match ip.into() {
            IpAddr::V4(ip) => self.inner.get_ref().assign_v4(ip, prefix_len),
            IpAddr::V6(ip) => self.inner.get_ref().assign_v6(ip, prefix_len),
        }
    }

    /// Adds entry to the routing table.
    /// # Panics
    /// If `ip` and `gateway` have different ip versions.
    pub fn route(&self, ip: impl Into<IpAddr>, prefix_len: u32, metric: u16) -> io::Result<()> {
        match ip.into() {
            IpAddr::V4(ip) => self.inner.get_ref().route_v4(ip, prefix_len, metric),
            IpAddr::V6(_ip) => todo!(),
        }
    }

    /// Splits interface into read and write halfs.
    pub fn split(self) -> (half::WriteHalf, half::ReadHalf) {
        let inner = Arc::new(AsyncFd::new(self.inner.into_inner()).unwrap());

        let write = half::WriteHalf {
            inner: inner.clone(),
        };
        let read = half::ReadHalf { inner };

        (write, read)
    }

    pub(crate) fn with_options(
        name: Option<&str>,
        mode: Mode,
        packet_info: bool,
        mtu: u16,
    ) -> crate::Result<Self> {
        let inner = sys::Inner::with_options(name, mode, packet_info, mtu)?;
        let mut flags = unsafe { libc::fcntl(inner.fd, libc::F_GETFL, 0) };
        if flags == -1 {
            return Err(Error::last());
        }

        flags |= libc::O_NONBLOCK;

        let err = unsafe { libc::fcntl(inner.fd, libc::F_SETFL, flags) };
        if err < 0 {
            return Err(Error::last());
        }

        Ok(Self {
            inner: AsyncFd::new(inner)?,
        })
    }
}

impl AsyncRead for Interface {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| inner.get_ref().read(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for Interface {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.inner.poll_write_ready(cx))?;

            match guard.try_io(|inner| inner.get_ref().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
