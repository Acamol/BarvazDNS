use std::io;
use std::time::Duration;
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::time::timeout;
use tokio::io::AsyncReadExt;

pub trait NamedPipeServerWithTimeout {
    async fn read_with_timeout(&mut self, buf: &mut [u8], duration: Duration) -> io::Result<usize>;
    async fn connect_with_timeout(&self, duration: Duration) -> io::Result<()>;
}

impl NamedPipeServerWithTimeout for NamedPipeServer {
    async fn read_with_timeout(&mut self, buf: &mut [u8], duration: Duration) -> io::Result<usize> {
        timeout(duration, self.read(buf))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Read operation timed out"))?
    }

    async fn connect_with_timeout(&self, duration: Duration) -> io::Result<()> {
        timeout(duration, self.connect()).await?
    }
}