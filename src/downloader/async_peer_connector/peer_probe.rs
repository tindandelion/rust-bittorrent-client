use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll, Waker},
};

type ProbeResult = io::Result<std::net::TcpStream>;

pub struct PeerProbe {
    pub addr: SocketAddr,
    fut: Pin<Box<dyn Future<Output = ProbeResult>>>,
    result: Option<ProbeResult>,
}

impl PeerProbe {
    pub fn new(
        addr: SocketAddr,
        fut: impl Future<Output = ProbeResult> + 'static,
    ) -> io::Result<Self> {
        let boxed = Box::new(fut);
        Ok(Self {
            addr,
            result: None,
            fut: Box::into_pin(boxed),
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
