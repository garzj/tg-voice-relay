use std::io;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct AHMConnection {
    stream: TcpStream,
}

impl AHMConnection {
    pub async fn connect(address: &str) -> io::Result<Self> {
        let address = address.to_owned();

        let stream = TcpStream::connect(&address).await?;

        Ok(AHMConnection { stream })
    }

    pub async fn write_preset(&mut self, preset: u16) -> io::Result<()> {
        let z_preset = preset - 1;
        let bank: u8 = (z_preset / 128) as u8;
        let ss: u8 = (z_preset % 128) as u8;
        let preset = vec![0xf0, 0xb0, 0x00, bank, 0xc0, ss];
        self.stream.write_all(&preset).await?;
        self.stream.flush().await?;

        let buf = &mut [0u8; 5];
        self.stream.read_exact(buf).await?;

        Ok(())
    }
}
