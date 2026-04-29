use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use super::reactor;

pub struct ConnectFuture {
    id: usize,
    addr: SocketAddr,
    stream: Option<mio::net::TcpStream>,
}

impl ConnectFuture {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            id: reactor::next_id(),
            addr,
            stream: None,
        }
    }
}

impl Future for ConnectFuture {
    type Output = io::Result<mio::net::TcpStream>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.stream.is_none() {
            let mut stream = mio::net::TcpStream::connect(self.addr)?;
            reactor::register_source(self.id, &mut stream, mio::Interest::WRITABLE)?;
            reactor::set_waker(self.id, cx.waker());
            self.stream = Some(stream);
        }

        let mut stream = self.stream.take().expect("the stream should be set");
        match stream.peer_addr() {
            Err(err) if err.kind() == io::ErrorKind::NotConnected => {
                self.stream = Some(stream);
                reactor::set_waker(self.id, cx.waker());
                Poll::Pending
            }
            Ok(_) => {
                reactor::deregister_source(self.id, &mut stream)?;
                Poll::Ready(Ok(stream))
            }
            Err(err) => {
                reactor::deregister_source(self.id, &mut stream)?;
                let error = stream.take_error().unwrap_or(None).unwrap_or(err);
                Poll::Ready(Err(error))
            }
        }
    }
}

impl Drop for ConnectFuture {
    fn drop(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            reactor::deregister_source(self.id, &mut stream).unwrap();
        }
    }
}
