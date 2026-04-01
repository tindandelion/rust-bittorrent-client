use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use tracing::{Span, debug_span};

pub struct PeerProbe {
    pub addr: SocketAddr,
    span: Span,
    fut: Pin<Box<dyn Future<Output = io::Result<std::net::TcpStream>>>>,
    result: Option<io::Result<std::net::TcpStream>>,
}

impl PeerProbe {
    pub fn connect(id: usize, addr: SocketAddr) -> io::Result<Self> {
        let span = debug_span!("connect_to_peer", addr = %addr);
        let fut = connect(id, addr);

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

async fn connect(id: usize, addr: SocketAddr) -> io::Result<std::net::TcpStream> {
    let stream = futures::ConnectFuture::new(id, addr).await?;
    let std_stream: std::net::TcpStream = stream.into();
    std_stream.set_nonblocking(false)?;
    Ok(std_stream)
}

mod futures {
    use std::{
        io,
        net::SocketAddr,
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    use tracing::debug;

    use crate::downloader::async_peer_connector::runtime;

    pub struct ConnectFuture {
        id: usize,
        addr: SocketAddr,
        stream: Option<mio::net::TcpStream>,
    }

    impl ConnectFuture {
        pub fn new(id: usize, addr: SocketAddr) -> Self {
            Self {
                id,
                addr,
                stream: None,
            }
        }

        pub fn my_poll(&mut self, waker: &Waker) -> Poll<io::Result<mio::net::TcpStream>> {
            if self.stream.is_none() {
                debug!("initiating connection");

                let mut stream = mio::net::TcpStream::connect(self.addr)?;
                runtime::register_source(
                    &mut stream,
                    self.id,
                    mio::Interest::WRITABLE | mio::Interest::READABLE,
                )?;
                runtime::set_waker(self.id, waker);
                self.stream = Some(stream);
            }

            let mut stream = self.stream.take().expect("the stream should be set");
            match stream.peer_addr() {
                Err(err) if err.kind() == io::ErrorKind::NotConnected => {
                    self.stream = Some(stream);
                    runtime::set_waker(self.id, waker);
                    Poll::Pending
                }
                Ok(_) => {
                    runtime::deregister_source(self.id, &mut stream)?;
                    Poll::Ready(Ok(stream))
                }
                Err(err) => {
                    runtime::deregister_source(self.id, &mut stream)?;
                    Poll::Ready(Err(err))
                }
            }
        }
    }

    impl Future for ConnectFuture {
        type Output = io::Result<mio::net::TcpStream>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.get_mut().my_poll(cx.waker())
        }
    }

    impl Drop for ConnectFuture {
        fn drop(&mut self) {
            if let Some(mut stream) = self.stream.take() {
                runtime::deregister_source(self.id, &mut stream).unwrap();
            }
        }
    }
}
