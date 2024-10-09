#[cfg(unix)]
use crate::platform::unix as sys;
use crate::Mode;
use std::{
    io::{self, Read, Write},
    net::IpAddr,
};

/// Blocking Tun/Tap interface
pub struct Interface {
    pub(crate) inner: sys::Inner,
}

impl Interface {
    /// Returns the name of the interface.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Brings the interface up.
    pub fn up(&self) -> io::Result<()> {
        self.inner.up()
    }

    /// Assigns ip address to this interface.
    /// # Panics
    /// If `ip` and `mask` have different versions.
    pub fn assign(&self, ip: impl Into<IpAddr>, prefix_len: u32) -> io::Result<()> {
        match ip.into() {
            IpAddr::V4(ip) => self.inner.assign_v4(ip, prefix_len),
            IpAddr::V6(ip) => self.inner.assign_v6(ip, prefix_len),
        }
    }

    pub(crate) fn with_options(
        name: Option<&str>,
        mode: Mode,
        packet_info: bool,
        mtu: u16,
    ) -> crate::Result<Self> {
        sys::Inner::with_options(name, mode, packet_info, mtu).map(|inner| Self { inner })
    }
}

impl Read for Interface {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for Interface {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
