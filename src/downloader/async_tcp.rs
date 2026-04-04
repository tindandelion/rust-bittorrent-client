use std::{io, net::SocketAddr, time::Duration};

use super::peer_comm::AsyncReadExact;

mod futures;
mod reactor;

pub fn poll_reactor(timeout: Option<Duration>) -> io::Result<bool> {
    reactor::poll(timeout)
}

#[derive(Debug)]
pub struct AsyncTcpStream {
    inner: mio::net::TcpStream,
}

impl AsyncTcpStream {
    pub async fn connect(addr: SocketAddr) -> io::Result<Self> {
        let stream = futures::ConnectFuture::new(addr).await?;
        Ok(Self { inner: stream })
    }
}

impl io::Write for AsyncTcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl AsyncReadExact for AsyncTcpStream {
    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = io::Result<()>> {
        futures::ReadExactFuture::new(&mut self.inner, buf)
    }
}

impl TryFrom<AsyncTcpStream> for std::net::TcpStream {
    type Error = io::Error;

    fn try_from(stream: AsyncTcpStream) -> Result<Self, Self::Error> {
        let std_stream = std::net::TcpStream::from(stream.inner);
        std_stream.set_nonblocking(false)?;
        Ok(std_stream)
    }
}

impl std::fmt::Display for AsyncTcpStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AsyncTcpStream(peer_addr: {})",
            self.inner
                .peer_addr()
                .map(|addr| addr.to_string())
                .unwrap_or("<unknown>".to_string())
        )
    }
}
