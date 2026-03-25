use std::{io, net::SocketAddr};

use crate::downloader::{
    peer_comm::{HandshakeMessage, PeerMessage},
    peer_connector::{message_buffer::MessageBuffer, probe_state::PeerStream},
};

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

    fn receive_handshake(&mut self) -> io::Result<HandshakeMessage> {
        HandshakeMessage::receive(&mut self.inner)
    }

    fn receive_message(&mut self) -> io::Result<PeerMessage> {
        self.buffer.read(&mut self.inner)
    }

    fn send_message(&mut self, msg: PeerMessage) -> io::Result<()> {
        msg.send(&mut self.inner)
    }
}
