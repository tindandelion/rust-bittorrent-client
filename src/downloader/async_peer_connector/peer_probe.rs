use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use mio::net::TcpStream;
use tracing::instrument;

use crate::downloader::{
    async_peer_connector::futures::ReadExactFuture, peer_comm::HandshakeMessage,
};

use super::futures::ConnectFuture;

pub struct PeerProbe {
    pub addr: SocketAddr,
    fut: Pin<Box<dyn Future<Output = io::Result<std::net::TcpStream>>>>,
    result: Option<io::Result<std::net::TcpStream>>,
}

impl PeerProbe {
    pub fn connect(addr: SocketAddr, handshake: HandshakeMessage) -> io::Result<Self> {
        Ok(Self {
            addr,
            result: None,
            fut: Box::pin(connect_to_peer(addr, handshake)),
        })
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.result, Some(Ok(_)))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.result, Some(Err(_)))
    }

    pub fn poll(&mut self, waker: &Waker) {
        // TODO: Why do we need this check?
        if self.result.is_some() {
            return;
        }
        let mut context = Context::from_waker(waker);

        match self.fut.as_mut().poll(&mut context) {
            Poll::Ready(res) => self.result = Some(res),
            Poll::Pending => {}
        }
    }
}

impl TryFrom<PeerProbe> for std::net::TcpStream {
    type Error = io::Error;

    fn try_from(probe: PeerProbe) -> Result<Self, Self::Error> {
        if let Some(Ok(stream)) = probe.result {
            Ok(stream)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "peer not connected"))
        }
    }
}

#[instrument(skip(handshake))]
async fn connect_to_peer(
    addr: SocketAddr,
    handshake: HandshakeMessage,
) -> io::Result<std::net::TcpStream> {
    let mut stream = init_connection(addr).await?;

    handshake.send(&mut stream)?;
    read_handshake(&mut stream).await?;

    let std_stream: std::net::TcpStream = stream.into();
    std_stream.set_nonblocking(false)?;
    Ok(std_stream)
}

#[instrument(skip(addr), err, ret)]
async fn init_connection(addr: SocketAddr) -> io::Result<TcpStream> {
    ConnectFuture::new(addr).await
}

#[instrument(skip(stream), err, ret)]
async fn read_handshake(stream: &mut TcpStream) -> io::Result<HandshakeMessage> {
    let mut buffer = [0; HandshakeMessage::SIZE];
    ReadExactFuture::new(stream, &mut buffer).await?;
    HandshakeMessage::receive(&mut &buffer[..])
}
