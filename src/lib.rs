//! Titan - Tun/Tap devices.

mod error;
pub use error::*;

/// Blocking interface
pub mod blocking;

/// Non blocking interface
#[cfg(unix)]
pub mod nonblocking;

mod platform {
    #[cfg(unix)]
    pub mod unix;
    #[cfg(unix)]
    pub(crate) use unix::*;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Interface mode
pub enum Mode {
    /// Tun interface, for IP packets
    Tun,
    /// Tap interface, for Ethernet packets.
    Tap,
}

/// Builder for the interface
pub struct InterfaceBuilder<'a> {
    name: Option<&'a str>,
    mode: Mode,
    packet_info: bool,
    mtu: u16,
}

impl<'a> InterfaceBuilder<'a> {
    /// Creates new interface builder with optimal settings.
    /// Default values are:
    /// * packet_info: `true`
    /// * name: `None`.
    pub fn new(mode: Mode) -> Self {
        Self {
            packet_info: true,
            name: None,
            mode,
            mtu: 1420,
        }
    }

    /// Sets MTU for the interface, default value is `1420`.
    pub fn mtu(mut self, mtu: u16) -> Self {
        self.mtu = mtu;
        self
    }

    /// Clears the name of the interface.
    pub fn unnamed(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets name for the interface.
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Returns if packet info should be included.
    pub fn packet_info(&self) -> bool {
        self.packet_info
    }

    /// Changes whether packet info should be included or not.
    pub fn with_packet_info(mut self, packet_info: bool) -> Self {
        self.packet_info = packet_info;
        self
    }

    /// Creates new blocking interface.
    pub fn blocking(self) -> crate::Result<blocking::Interface> {
        blocking::Interface::with_options(self.name, self.mode, self.packet_info, self.mtu)
    }

    /// Creates new async interface.
    pub fn nonblocking(self) -> crate::Result<nonblocking::Interface> {
        nonblocking::Interface::with_options(self.name, self.mode, self.packet_info, self.mtu)
    }
}
