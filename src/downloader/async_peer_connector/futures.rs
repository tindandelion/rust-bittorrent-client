use super::reactor;
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::debug;

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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let waker = cx.waker();

        if this.stream.is_none() {
            debug!("initiating connection");

            let mut stream = mio::net::TcpStream::connect(this.addr)?;
            reactor::register_source(
                this.id,
                &mut stream,
                mio::Interest::WRITABLE | mio::Interest::READABLE,
            )?;
            reactor::set_waker(this.id, waker);
            this.stream = Some(stream);
        }

        let mut stream = this.stream.take().expect("the stream should be set");
        match stream.peer_addr() {
            Err(err) if err.kind() == io::ErrorKind::NotConnected => {
                this.stream = Some(stream);
                reactor::set_waker(this.id, waker);
                Poll::Pending
            }
            Ok(_) => {
                reactor::deregister_source(this.id, &mut stream)?;
                Poll::Ready(Ok(stream))
            }
            Err(err) => {
                reactor::deregister_source(this.id, &mut stream)?;
                Poll::Ready(Err(err))
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
