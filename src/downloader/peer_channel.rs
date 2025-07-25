use std::{
    error::Error,
    io,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::types::{PeerId, Sha1};

use super::file_downloader::{Block, DownloadChannel};
use super::handshake_message::HandshakeMessage;
use super::peer_messages::PeerMessage;

pub struct PeerChannel {
    stream: TcpStream,
}

impl PeerChannel {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    const MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn connect(addr: &SocketAddr) -> Result<PeerChannel, Box<dyn Error>> {
        let stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        stream.set_read_timeout(Some(Self::MESSAGE_READ_TIMEOUT))?;
        Ok(PeerChannel { stream })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn handshake(&mut self, info_hash: Sha1, peer_id: PeerId) -> Result<String, io::Error> {
        HandshakeMessage::new(info_hash, peer_id).send(&mut self.stream)?;

        let current_timeout = self.stream.read_timeout()?;
        self.stream
            .set_read_timeout(Some(Self::HANDSHAKE_TIMEOUT))?;
        let response = HandshakeMessage::receive(&mut self.stream);
        self.stream.set_read_timeout(current_timeout)?;

        response.map(|msg| String::from_utf8_lossy(&msg.peer_id).to_string())
    }

    pub fn receive_bitfield(&mut self) -> io::Result<Vec<u8>> {
        match PeerMessage::receive(&mut self.stream)? {
            PeerMessage::Bitfield(bitfield) => Ok(bitfield),
            other => error_unexpected_message("bitfield", &other),
        }
    }

    pub fn send_interested(&mut self) -> io::Result<()> {
        PeerMessage::Interested.send(&mut self.stream)
    }

    pub fn receive_unchoke(&mut self) -> io::Result<()> {
        match PeerMessage::receive(&mut self.stream)? {
            PeerMessage::Unchoke => Ok(()),
            other => error_unexpected_message("unchoke", &other),
        }
    }
}

impl DownloadChannel for PeerChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        PeerMessage::Request {
            piece_index,
            offset,
            length,
        }
        .send(&mut self.stream)
    }

    fn receive(&mut self) -> io::Result<Block> {
        let msg = PeerMessage::receive(&mut self.stream)?;
        match msg {
            PeerMessage::Piece {
                piece_index,
                offset,
                block,
            } => Ok(Block {
                piece_index,
                offset,
                data: block,
            }),
            other => error_unexpected_message("piece", &other),
        }
    }
}

fn error_unexpected_message<T>(expected: &str, received: &PeerMessage) -> io::Result<T> {
    Err(io::Error::other(format!(
        "Expected `{}` message, received: {:?}",
        expected, received
    )))
}
