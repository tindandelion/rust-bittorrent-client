use std::{io, net::SocketAddr};

use mio::Token;
use tracing::{Span, debug, debug_span, warn};

use crate::downloader::async_peer_connector::{
    probe_state::{ProbeContext, ProbeError, ProbeState},
    runtime,
};

pub struct PeerProbe {
    stream: mio::net::TcpStream,
    state: ProbeState,
    pub addr: SocketAddr,
    span: Span,
    id: Token,
}

impl PeerProbe {
    pub fn connect(addr: SocketAddr, context: ProbeContext) -> io::Result<Self> {
        let span = debug_span!("connect_to_peer", addr = %addr);
        let stream = span.in_scope(|| {
            debug!("initiating connection");
            mio::net::TcpStream::connect(addr)
        })?;
        let id = runtime::next_id();
        Ok(Self {
            id,
            stream,
            state: ProbeState::Connecting(context),
            addr,
            span,
        })
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state, ProbeState::Handshaking(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.state, ProbeState::Error)
    }

    pub fn register(&mut self) -> io::Result<Token> {
        runtime::register_stream(
            &mut self.stream,
            self.id,
            mio::Interest::WRITABLE | mio::Interest::READABLE,
        )?;
        Ok(self.id)
    }

    pub fn unregister(&mut self) -> io::Result<()> {
        runtime::deregister_stream(&mut self.stream)
    }

    pub fn handle_event(&mut self) {
        let _guard = self.span.enter();

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
}

impl TryFrom<PeerProbe> for std::net::TcpStream {
    type Error = io::Error;

    fn try_from(value: PeerProbe) -> Result<Self, Self::Error> {
        if !value.is_connected() {
            return Err(io::Error::new(io::ErrorKind::Other, "peer not connected"));
        }
        let std_stream: std::net::TcpStream = value.stream.into();
        std_stream.set_nonblocking(false)?;
        Ok(std_stream)
    }
}
