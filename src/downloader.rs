mod handshake_message;
mod peer_messages;
mod piece_downloader;

use std::{
    error::Error,
    io::{self},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::types::{PeerId, Sha1};
use handshake_message::HandshakeMessage;
use peer_messages::PeerMessage;

pub struct FileDownloader {
    stream: TcpStream,
}

impl FileDownloader {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn connect(addr: &SocketAddr) -> Result<FileDownloader, Box<dyn Error>> {
        let stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        stream.set_read_timeout(Some(Self::READ_TIMEOUT))?;
        Ok(FileDownloader { stream })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn handshake(
        &mut self,
        info_hash: Sha1,
        peer_id: PeerId,
    ) -> Result<String, Box<dyn Error>> {
        HandshakeMessage::new(info_hash, peer_id).send(&mut self.stream)?;
        let response = HandshakeMessage::receive(&mut self.stream)?;
        Ok(String::from_utf8_lossy(&response.peer_id).to_string())
    }

    pub fn receive_bitfield(&mut self) -> io::Result<Vec<u8>> {
        match PeerMessage::receive(&mut self.stream)? {
            PeerMessage::Bitfield(bitfield) => Ok(bitfield),
            _ => error_unexpected_message("bitfield"),
        }
    }

    pub fn send_interested(&mut self) -> io::Result<()> {
        PeerMessage::Interested.send(&mut self.stream)
    }

    pub fn receive_unchoke(&mut self) -> io::Result<()> {
        match PeerMessage::receive(&mut self.stream)? {
            PeerMessage::Unchoke => Ok(()),
            _ => error_unexpected_message("unchoke"),
        }
    }

    pub fn receive_piece(&mut self) -> io::Result<(u32, u32, Vec<u8>)> {
        match PeerMessage::receive(&mut self.stream)? {
            PeerMessage::Piece {
                piece_index,
                offset,
                block,
            } => Ok((piece_index, offset, block)),
            _ => error_unexpected_message("piece"),
        }
    }

    pub fn request_block(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        PeerMessage::Request {
            piece_index,
            offset,
            length,
        }
        .send(&mut self.stream)
    }
}

fn error_unexpected_message<T>(expected: &str) -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Expected `{}` message", expected),
    ))
}
