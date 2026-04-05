use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use crate::downloader::PeerChannel;

use super::probe_result::ProbeResult;

pub struct PeerProbe {
    pub addr: SocketAddr,
    fut: Pin<Box<dyn Future<Output = ProbeResult<PeerChannel>>>>,
    result: Option<ProbeResult<PeerChannel>>,
}

impl PeerProbe {
    pub fn new(
        addr: SocketAddr,
        fut: impl Future<Output = ProbeResult<PeerChannel>> + 'static,
    ) -> io::Result<Self> {
        Ok(Self {
            addr,
            result: None,
            fut: Box::pin(fut),
        })
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.result, Some(Ok(_)))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.result, Some(Err(_)))
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) {
        match self.fut.as_mut().poll(cx) {
            Poll::Ready(res) => self.result = Some(res),
            Poll::Pending => {}
        }
    }
}

impl TryFrom<PeerProbe> for PeerChannel {
    type Error = io::Error;

    fn try_from(probe: PeerProbe) -> Result<Self, Self::Error> {
        if let Some(Ok(channel)) = probe.result {
            Ok(channel)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "peer not connected"))
        }
    }
}
