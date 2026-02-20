use std::{
    io,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::{
    downloader::MessageChannel,
    types::{PeerId, Sha1},
};

use super::{PeerMessage, handshake_message::HandshakeMessage};

pub struct PeerChannel {
    peer_addr: SocketAddr,
    remote_id: PeerId,
    stream: TcpStream,
}

impl PeerChannel {
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    const MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn handshake(
        mut stream: TcpStream,
        info_hash: &Sha1,
        peer_id: &PeerId,
    ) -> io::Result<PeerChannel> {
        stream.set_read_timeout(Some(Self::HANDSHAKE_TIMEOUT))?;
        let remote_id = Self::exchange_handshake(&mut stream, info_hash, peer_id)?;
        stream.set_read_timeout(Some(Self::MESSAGE_READ_TIMEOUT))?;
        let peer_addr = stream.peer_addr()?;

        Ok(PeerChannel {
            stream,
            remote_id,
            peer_addr,
        })
    }

    pub fn peer_addr(&self) -> &SocketAddr {
        &self.peer_addr
    }

    pub fn remote_id(&self) -> &PeerId {
        &self.remote_id
    }

    fn exchange_handshake(
        stream: &mut TcpStream,
        info_hash: &Sha1,
        peer_id: &PeerId,
    ) -> io::Result<PeerId> {
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
