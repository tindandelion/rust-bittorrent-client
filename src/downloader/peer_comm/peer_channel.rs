use std::{
    io,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::types::PeerId;

use super::PeerMessage;

pub struct PeerChannel {
    peer_addr: SocketAddr,
    remote_id: PeerId,
    pub stream: TcpStream,
}

impl PeerChannel {
    const MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

    pub fn from_stream(stream: TcpStream, remote_id: PeerId) -> io::Result<PeerChannel> {
        let peer_addr = stream.peer_addr()?;
        stream.set_read_timeout(Some(Self::MESSAGE_READ_TIMEOUT))?;
        Ok(PeerChannel {
            stream,
            remote_id,
            peer_addr,
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn remote_id(&self) -> PeerId {
        self.remote_id
    }

    pub fn receive(&mut self) -> io::Result<PeerMessage> {
        PeerMessage::receive(&mut self.stream)
    }

    pub fn send(&mut self, msg: &PeerMessage) -> io::Result<()> {
        msg.send(&mut self.stream)
    }
}
