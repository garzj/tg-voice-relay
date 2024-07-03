use std::io;

use tokio::{io::AsyncWriteExt, net::TcpStream};

pub struct AHMConnection {
    address: String,
    stream: TcpStream,
}

impl AHMConnection {
    pub async fn connect(address: &str) -> io::Result<Self> {
        let address = address.to_owned();

        let stream = TcpStream::connect(&address).await?;

        Ok(AHMConnection { address, stream })
    }

    pub async fn write_preset(&mut self, preset: u16) -> io::Result<()> {
        let z_preset = preset - 1;
        let bank: u8 = (z_preset / 128) as u8;
        let ss: u8 = (z_preset % 128) as u8;
        let preset = vec![0xf0, 0xb0, 0x00, bank, 0xc0, ss];
        self.stream.write_all(&preset).await
    }
}
