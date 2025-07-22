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
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    const MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn connect(addr: &SocketAddr) -> Result<FileDownloader, Box<dyn Error>> {
        let stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        stream.set_read_timeout(Some(Self::MESSAGE_READ_TIMEOUT))?;
        Ok(FileDownloader { stream })
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

    pub fn download_file(
        &mut self,
        piece_hashes: Vec<Sha1>,
        piece_length: u32,
        file_length: usize,
    ) -> io::Result<Vec<u8>> {
        let channel = TcpPieceDownloadChannel::new(&mut self.stream);
        let mut downloader = PieceDownloader::new(channel, piece_hashes, piece_length, file_length);
        let data = downloader.download_all()?;
        Ok(data)
    }

    pub fn download_piece(
        &mut self,
        piece_hashes: Vec<Sha1>,
        piece_index: u32,
        piece_length: u32,
        file_length: usize,
    ) -> io::Result<Piece> {
        let channel = TcpPieceDownloadChannel::new(&mut self.stream);
        let mut downloader = PieceDownloader::new(channel, piece_hashes, piece_length, file_length);
        let piece = downloader.download_piece(piece_index, piece_length)?;
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
}

impl<'a> TcpPieceDownloadChannel<'a> {
    pub fn new(stream: &'a mut TcpStream) -> Self {
        Self { stream }
    }
}

impl PieceDownloadChannel for TcpPieceDownloadChannel<'_> {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        PeerMessage::Request {
            piece_index: piece_index,
            offset,
            length,
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
            } => Ok(piece_downloader::Block {
                piece_index,
                offset,
                data: block,
            }),
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
