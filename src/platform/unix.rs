use crate::{Error, Mode};
use libc::*;
use std::{
    ffi::CStr,
    io, mem,
    net::{Ipv4Addr, Ipv6Addr},
    os::fd::{AsRawFd, RawFd},
};

const TUNSETIFF: u64 = 1074025674;

pub(crate) struct Inner {
    name: [std::ffi::c_char; IFNAMSIZ],
    pub(crate) fd: i32,
}

impl AsRawFd for Inner {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for Inner {
    #[inline]
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

macro_rules! psockaddr {
    ($val:expr) => {
        (*(&mut $val as *mut libc::sockaddr as *mut libc::sockaddr_in))
    };
}

impl Inner {
    pub(crate) fn with_options(
        name: Option<&str>,
        mode: Mode,
        packet_info: bool,
        mtu: u16,
    ) -> crate::Result<Self> {
        #[cfg(target_os = "linux")]
        const TUN: &str = "/dev/net/tun\0";
        #[cfg(not(target_os = "linux"))]
        const TUN: &str = "/dev/tun\0";

        let fd = unsafe { open(TUN.as_ptr().cast::<c_char>(), O_RDWR) };
        let mut ifr: ifreq = unsafe { mem::zeroed() };
        ifr.ifr_name = to_ifr_name(name)?;

        match mode {
            Mode::Tun => ifr.ifr_ifru.ifru_flags = IFF_TUN as i16,
            Mode::Tap => ifr.ifr_ifru.ifru_flags = IFF_TAP as i16,
        }
        if packet_info {
            unsafe { ifr.ifr_ifru.ifru_flags |= IFF_NO_PI as i16 };
        }

        let err = unsafe { ioctl(fd, TUNSETIFF as _, &mut ifr as *mut ifreq) };
        if err < 0 {
            unsafe { close(fd) };
            return Err(Error::last());
        }

        let skfd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_IP) };
        if skfd < 0 {
            unsafe { close(fd) };
            return Err(Error::last());
        }

        let mut mtuifr: ifreq = unsafe { mem::zeroed() };
        mtuifr.ifr_name = to_ifr_name(name)?;
        mtuifr.ifr_ifru.ifru_mtu = mtu as _;

        let err = unsafe { ioctl(skfd, SIOCSIFMTU as _, &mut mtuifr as *mut ifreq) };
        unsafe { close(skfd) };

        if err < 0 {
            unsafe { close(skfd) };
            return Err(Error::last());
        }

        Ok(Self {
            name: ifr.ifr_name,
            fd,
        })
    }

    pub(crate) fn assign_v6(&self, ip: Ipv6Addr, prefix_len: u32) -> io::Result<()> {
        let fd = unsafe { socket(AF_INET6, SOCK_DGRAM, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut sa: sockaddr_in6 = unsafe { mem::zeroed() };
        sa.sin6_family = AF_INET6 as _;
        sa.sin6_addr = in6_addr {
            s6_addr: ip.octets(),
        };

        let mut ifr: in6_ifreq = unsafe { mem::zeroed() };
        ifr.ifr6_ifindex = unsafe { if_nametoindex(self.name.as_ptr()) } as _;
        ifr.ifr6_prefixlen = prefix_len;
        ifr.ifr6_addr.s6_addr = ip.octets();

        let err = unsafe { ioctl(fd, SIOCSIFADDR as _, &mut ifr) };
        unsafe { close(fd) };

        if err < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub(crate) fn assign_v4(&self, ip: Ipv4Addr, prefix_len: u32) -> io::Result<()> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_IP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut ifr: ifreq = unsafe { mem::zeroed() };
        ifr.ifr_name.copy_from_slice(&self.name);
        ifr.ifr_ifru.ifru_addr.sa_family = AF_INET as u16;

        let mut ifrc = ifr;
        unsafe {
            psockaddr!(ifrc.ifr_ifru.ifru_addr).sin_family = AF_INET as _;
            psockaddr!(ifrc.ifr_ifru.ifru_addr).sin_addr.s_addr = u32::from_le_bytes(ip.octets());
        }

        let err = unsafe { ioctl(fd, SIOCSIFADDR as _, &mut ifrc as *mut ifreq) };
        if err < 0 {
            unsafe { close(fd) };
            return Err(io::Error::last_os_error());
        }

        let mut ifrc = ifr;
        unsafe {
            psockaddr!(ifrc.ifr_ifru.ifru_addr).sin_family = AF_INET as _;
            psockaddr!(ifrc.ifr_ifru.ifru_addr).sin_addr.s_addr = 0xFFFFFFFF >> (32 - prefix_len);
        }

        let err = unsafe { ioctl(fd, SIOCSIFNETMASK as _, &mut ifrc as *mut ifreq) };
        unsafe { close(fd) };

        if err < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub(crate) fn route_v4(&self, ip: Ipv4Addr, prefix_len: u32, metric: u16) -> io::Result<()> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_IP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut route: rtentry = unsafe { mem::zeroed() };

        unsafe {
            // why 0xFFFFFFFF_u32 >> 32 is an error ?????????????
            let netmask = (0xFFFFFFFF_u64 >> (32 - prefix_len)) as u32;

            psockaddr!(route.rt_dst).sin_family = AF_INET as _;
            psockaddr!(route.rt_dst).sin_addr.s_addr = u32::from_le_bytes(ip.octets()) & netmask;
            psockaddr!(route.rt_genmask).sin_family = AF_INET as _;
            psockaddr!(route.rt_genmask).sin_addr.s_addr = netmask;
        }

        let mut ifname = self.name;
        route.rt_dev = ifname.as_mut_ptr();
        route.rt_metric = metric as i16;

        let err = unsafe { ioctl(fd, SIOCADDRT as _, &mut route) };
        unsafe { close(fd) };

        if err < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub(crate) fn up(&self) -> io::Result<()> {
        let fd = unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_IP) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut ifr: ifreq = unsafe { mem::zeroed() };

        ifr.ifr_name.copy_from_slice(&self.name);
        unsafe { ifr.ifr_ifru.ifru_flags |= IFF_UP as i16 }

        let err = unsafe { ioctl(fd, SIOCSIFFLAGS as _, &mut ifr as *mut ifreq) };
        unsafe { close(fd) };

        if err < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub(crate) fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let err = unsafe { read(self.fd, buf.as_mut_ptr().cast::<c_void>(), buf.len()) };
        if err < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(err as usize)
        }
    }

    pub(crate) fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let err = unsafe { write(self.fd, buf.as_ptr().cast::<c_void>(), buf.len()) };
        if err < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(err as usize)
        }
    }

    pub(crate) fn name(&self) -> &str {
        unsafe { CStr::from_ptr(self.name.as_ptr()).to_str().unwrap() }
    }
}

fn to_ifr_name(name: Option<&str>) -> crate::Result<[c_char; IFNAMSIZ]> {
    let mut ifrname = [0; IFNAMSIZ];

    if let Some(name) = name {
        if name.len() >= IFNAMSIZ || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::InvalidName);
        }

        // SAFETY: len is less than IFNAMSIZ and is valid ascii.
        unsafe {
            ifrname
                .as_mut_ptr()
                .copy_from_nonoverlapping(name.as_ptr().cast::<c_char>(), name.len());
        }
    }

    Ok(ifrname)
}
