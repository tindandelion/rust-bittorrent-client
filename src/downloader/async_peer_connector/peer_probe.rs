use std::{io, net::SocketAddr};

use mio::{Token, event::Event};
use tracing::{Span, debug, debug_span, trace, warn};

use crate::downloader::{
    PeerChannel,
    async_peer_connector::{
        mio_peer_stream::MioPeerStream,
        probe_state::{ProbeContext, ProbeError, ProbeState},
        runtime,
    },
};

pub struct PeerProbe {
    stream: MioPeerStream,
    pub state: ProbeState,
    pub addr: SocketAddr,
    span: Span,
}

impl PeerProbe {
    pub fn connect(addr: SocketAddr, context: ProbeContext) -> io::Result<Self> {
        let span = debug_span!("connect_to_peer", addr = %addr);
        let stream = span.in_scope(|| {
            debug!("initiating connection");
            mio::net::TcpStream::connect(addr).map(MioPeerStream::new)
        })?;
        Ok(Self {
            stream,
            state: ProbeState::Connecting(context),
            addr,
            span,
        })
    }

    pub fn register(&mut self) -> io::Result<Token> {
        let token = runtime::register_stream(
            &mut self.stream.inner,
            mio::Interest::WRITABLE | mio::Interest::READABLE,
        )?;
        Ok(token)
    }

    pub fn unregister(&mut self) -> io::Result<()> {
        runtime::deregister_stream(&mut self.stream.inner)
    }

    pub fn handle_event(&mut self, event: &Event) {
        let _guard = self.span.enter();
        trace!(?event, "received event");

        if event.is_error() {
            match self.stream.inner.take_error() {
                Ok(Some(err)) => debug!(?err, "probe error: I/O error"),
                Ok(None) => {}
                Err(err) => debug!(?err, "failed to take I/O error"),
            }
            self.state = ProbeState::Error;
            return;
        }

        loop {
            match self.state.update(&mut self.stream) {
                Ok(next_state) => {
                    self.state = next_state;
                    if self.state.is_terminal() {
                        break;
                    }
                }
                Err(ProbeError::IO(err)) if err.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ProbeError::IO(err)) => {
                    debug!(?err, "probe error: I/O error");
                    self.state = ProbeState::Error;
                    break;
                }
                Err(err) => {
                    warn!(?err, "probe error");
                    self.state = ProbeState::Error;
                    break;
                }
            }
        }
    }

    pub fn into_peer_channel(self) -> io::Result<PeerChannel> {
        match self.state {
            ProbeState::Unchoked(remote_id) => {
                let std_stream: std::net::TcpStream = self.stream.inner.into();
                std_stream.set_nonblocking(false)?;

                PeerChannel::from_stream(std_stream, remote_id)
            }
            _ => Err(std::io::Error::other("peer did not unchoke")),
        }
    }
}
