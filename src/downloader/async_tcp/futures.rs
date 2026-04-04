use super::reactor;
use std::io::Read;
use std::task::Waker;
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

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

pub struct ReadExactFuture<'a, 'b> {
    id: Option<usize>,
    stream: &'a mut mio::net::TcpStream,
    buffer: &'b mut [u8],
}

impl<'a, 'b> ReadExactFuture<'a, 'b> {
    pub fn new(stream: &'a mut mio::net::TcpStream, buffer: &'b mut [u8]) -> Self {
        Self {
            id: None,
            stream,
            buffer,
        }
    }

    fn register(&mut self, waker: &Waker) -> io::Result<()> {
        if self.id.is_none() {
            let id = reactor::next_id();
            reactor::register_source(id, self.stream, mio::Interest::READABLE)?;
            reactor::set_waker(id, waker);
            self.id = Some(id);
        }
        Ok(())
    }

    fn deregister(&mut self) -> io::Result<()> {
        if let Some(id) = self.id {
            reactor::deregister_source(id, self.stream)?;
            self.id = None;
        }
        Ok(())
    }
}

impl<'a, 'b> Future for ReadExactFuture<'a, 'b> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.id.is_none() {
            self.register(cx.waker())?;
        }

        let id = self.id.expect("the id should be set");
        let mut buf = vec![0; self.buffer.len()];

        match self.stream.read(&mut buf) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                reactor::set_waker(id, cx.waker());
                Poll::Pending
            }
            Ok(n) => {
                self.deregister()?;
                if n == buf.len() {
                    self.buffer.copy_from_slice(&buf);
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!(
                            "Not enough data has been received: expected {}, received {}",
                            buf.len(),
                            n
                        ),
                    )))
                }
            }
            Err(err) => {
                self.deregister()?;
                Poll::Ready(Err(err))
            }
        }
    }
}

impl<'a, 'b> Drop for ReadExactFuture<'a, 'b> {
    fn drop(&mut self) {
        self.deregister().unwrap();
    }
}
