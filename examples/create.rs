#![allow(missing_docs)]

use std::{error::Error, net::IpAddr};
use titan::{InterfaceBuilder, Mode};
use tokio::io::AsyncReadExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut iface = InterfaceBuilder::new(Mode::Tun)
        .with_name("foo")
        .nonblocking()?;
    eprintln!("Created interface: {}", iface.name());

    iface.assign("192.0.2.1".parse::<IpAddr>()?, 24)?;
    iface.up()?;

    iface.route("10.200.200.0".parse::<IpAddr>()?, 24, 2)?;

    let mut buf = [0; 1504];
    loop {
        let len = iface.read(&mut buf).await.unwrap();
        eprintln!("Received: {:x?}", &buf[..len]);
    }
}
