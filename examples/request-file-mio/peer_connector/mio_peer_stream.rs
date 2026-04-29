use io::Read;
use std::{
    io::{self, ErrorKind},
    net::SocketAddr,
};

use tracing::{Level, instrument, trace};

use bt_client::downloader::peer_comm::{HandshakeMessage, PeerMessage};

use crate::peer_connector::message_buffer::MessageBuffer;
use crate::peer_connector::probe_state::PeerStream;

pub struct MioPeerStream {
    pub inner: mio::net::TcpStream,
    buffer: MessageBuffer,
}

impl MioPeerStream {
    pub fn new(stream: mio::net::TcpStream) -> Self {
        Self {
            inner: stream,
            buffer: MessageBuffer::new(),
        }
    }
}

impl PeerStream for MioPeerStream {
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    fn send_handshake(&mut self, handshake: HandshakeMessage) -> io::Result<()> {
        handshake.send(&mut self.inner)
    }

    #[instrument(skip(self), err, level = Level::TRACE)]
    fn receive_handshake(&mut self) -> io::Result<HandshakeMessage> {
        let mut buffer = [0; HandshakeMessage::SIZE];
        let bytes_read = self.inner.read(&mut buffer)?;

        if bytes_read != HandshakeMessage::SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("did not receive the whole handshake message, got {bytes_read} bytes"),
            ));
        }

        HandshakeMessage::receive(&mut &buffer[..])
    }

    #[instrument(skip(self), err, level = Level::TRACE)]
    fn receive_message(&mut self) -> io::Result<PeerMessage> {
        trace!(buffer_len = self.buffer.len(), "receiving peer message");
        self.buffer
            .read(&mut self.inner)
            .inspect(|msg| trace!(?msg, "received peer message"))
            .inspect_err(|err| {
                if err.kind() == ErrorKind::WouldBlock {
                    trace!(
                        cur_buffer_len = self.buffer.len(),
                        "did not receive the whole message, will retry"
                    );
                } else {
                    trace!(?err, "error receiving peer message, flushing buffer");
                    self.buffer = MessageBuffer::new();
                }
            })
    }

    fn send_message(&mut self, msg: PeerMessage) -> io::Result<()> {
        msg.send(&mut self.inner)
    }
}
