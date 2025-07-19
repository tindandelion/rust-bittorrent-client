mod handshake_message;
mod peer_messages;
mod piece_downloader;

use std::{
    error::Error,
    io::{self},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::{
    downloader::piece_downloader::{PieceDownloadChannel, PieceDownloader},
    types::{PeerId, Sha1},
};
use handshake_message::HandshakeMessage;
use peer_messages::PeerMessage;

pub struct Piece(Vec<u8>);

pub struct FileDownloader {
    stream: TcpStream,
}

impl FileDownloader {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const READ_TIMEOUT: Duration = Duration::from_secs(10);
    const BLOCK_LENGTH: usize = 1 << 14;

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

    pub fn download_piece(&mut self, piece_index: u32, piece_length: usize) -> io::Result<Piece> {
        let channel = TcpPieceDownloadChannel::new(&mut self.stream, piece_index);
        let mut downloader = PieceDownloader::new(channel, piece_length, Self::BLOCK_LENGTH);
        let piece = downloader.download_piece()?;
        Ok(Piece(piece))
    }
}

fn error_unexpected_message<T>(expected: &str) -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Expected `{}` message", expected),
    ))
}

struct TcpPieceDownloadChannel<'a> {
    stream: &'a mut TcpStream,
    piece_index: u32,
}

impl<'a> TcpPieceDownloadChannel<'a> {
    pub fn new(stream: &'a mut TcpStream, piece_index: u32) -> Self {
        Self {
            stream,
            piece_index,
        }
    }
}

impl PieceDownloadChannel for TcpPieceDownloadChannel<'_> {
    fn request(&mut self, offset: usize, length: usize) -> io::Result<()> {
        PeerMessage::Request {
            piece_index: self.piece_index,
            offset: offset as u32,
            length: length as u32,
        }
        .send(self.stream)
    }

    fn receive(&mut self) -> io::Result<piece_downloader::Block> {
        let msg = PeerMessage::receive(self.stream)?;
        match msg {
            PeerMessage::Piece {
                piece_index,
                offset,
                block,
            } => {
                if piece_index != self.piece_index {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Unexpected piece index in response: expected {}, got {}",
                            self.piece_index, piece_index
                        ),
                    ));
                }
                Ok(piece_downloader::Block {
                    offset: offset as usize,
                    data: block,
                })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected message in response: expected `piece` message, got `{:?}`",
                    msg
                ),
            )),
        }
    }
}

impl Piece {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn verify_hash(&self, hash: &Sha1) -> bool {
        hash == &Sha1::calculate(&self.0)
    }
}
