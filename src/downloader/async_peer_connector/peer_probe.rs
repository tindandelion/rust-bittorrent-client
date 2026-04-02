use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use mio::net::TcpStream;
use tracing::{Span, debug_span};

use crate::downloader::{
    async_peer_connector::futures::ReadExactFuture, peer_comm::HandshakeMessage,
};

use super::futures::ConnectFuture;

pub struct PeerProbe {
    pub addr: SocketAddr,
    span: Span,
    fut: Pin<Box<dyn Future<Output = io::Result<std::net::TcpStream>>>>,
    result: Option<io::Result<std::net::TcpStream>>,
}

impl PeerProbe {
    pub fn connect(addr: SocketAddr, handshake: HandshakeMessage) -> io::Result<Self> {
        let span = debug_span!("connect_to_peer", addr = %addr);
        let fut = connect(addr, handshake);

        Ok(Self {
            addr,
            span,
            fut: Box::pin(fut),
            result: None,
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

        let _guard = self.span.enter();

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

async fn connect(addr: SocketAddr, handshake: HandshakeMessage) -> io::Result<std::net::TcpStream> {
    let mut stream = ConnectFuture::new(addr).await?;

    handshake.send(&mut stream)?;
    read_handshake(&mut stream).await?;

    let std_stream: std::net::TcpStream = stream.into();
    std_stream.set_nonblocking(false)?;
    Ok(std_stream)
}

async fn read_handshake(stream: &mut TcpStream) -> io::Result<HandshakeMessage> {
    let mut buffer = [0; HandshakeMessage::SIZE];
    ReadExactFuture::new(stream, &mut buffer).await?;
    HandshakeMessage::receive(&mut &buffer[..])
}
