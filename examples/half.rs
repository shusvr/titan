#![allow(missing_docs)]

use std::time::Duration;
use titan::{InterfaceBuilder, Mode, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    spawn,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let iface = InterfaceBuilder::new(Mode::Tun).nonblocking()?;
    eprintln!("Created interface: {}", iface.name());

    let (mut write, mut read) = iface.split();

    let packet = hex::decode("45000054fb3540004001296f0a0101020a01010108009bf80007000480eb0b660000000002d80e0000000000101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f3031323334353637").unwrap();

    spawn(async move {
        loop {
            write.write_all(&packet).await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    loop {
        let mut buf = [0; 1500];
        let len = read.read(&mut buf).await.unwrap();
        eprintln!("Recv: {:x?}", &buf[..len]);
    }
}
