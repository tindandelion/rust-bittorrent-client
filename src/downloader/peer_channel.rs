use std::{
    io,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::{
    downloader::peer_comm::{MessageChannel, PeerMessage},
    types::{PeerId, Sha1},
};

use super::file_downloader::{Block, DownloadChannel, RequestChannel};
use super::handshake_message::HandshakeMessage;

pub struct PeerChannel {
    peer_addr: SocketAddr,
    remote_id: PeerId,
    stream: TcpStream,
}

impl PeerChannel {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    const MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn connect(
        addr: &SocketAddr,
        info_hash: &Sha1,
        peer_id: &PeerId,
    ) -> io::Result<PeerChannel> {
        let mut stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        stream.set_read_timeout(Some(Self::HANDSHAKE_TIMEOUT))?;
        let remote_id = Self::handshake(&mut stream, info_hash, peer_id)?;
        stream.set_read_timeout(Some(Self::MESSAGE_READ_TIMEOUT))?;

        Ok(PeerChannel {
            stream,
            remote_id,
            peer_addr: *addr,
        })
    }

    pub fn peer_addr(&self) -> &SocketAddr {
        &self.peer_addr
    }

    pub fn remote_id(&self) -> &PeerId {
        &self.remote_id
    }

    fn handshake(stream: &mut TcpStream, info_hash: &Sha1, peer_id: &PeerId) -> io::Result<PeerId> {
        HandshakeMessage::new(info_hash, peer_id).send(stream)?;
        HandshakeMessage::receive(stream).map(|msg| PeerId::new(msg.peer_id))
    }
}

impl MessageChannel for PeerChannel {
    fn receive(&mut self) -> io::Result<PeerMessage> {
        PeerMessage::receive(&mut self.stream)
    }

    fn send(&mut self, msg: &PeerMessage) -> io::Result<()> {
        msg.send(&mut self.stream)
    }
}

impl RequestChannel for PeerChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        PeerMessage::Request {
            piece_index,
            offset,
            length,
        }
        .send(&mut self.stream)
    }
}

impl DownloadChannel for PeerChannel {
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
