use mio::{event::Event, net::TcpStream};
use tracing::debug;

use crate::{downloader::peer_comm::handshake_message::HandshakeMessage, types::PeerId};

#[derive(Clone, Copy)]
pub enum ProbeState {
    Connecting(HandshakeMessage),
    Handshaking(HandshakeMessage),
    Connected(PeerId),
    Error,
}

impl ProbeState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }

    pub fn handle_event(self, stream: &mut TcpStream, event: &Event) -> Self {
        match self {
            Self::Connecting(handshake) => Self::handle_connect(stream, handshake),
            Self::Handshaking(handshake) if event.is_readable() => {
                Self::handle_handshake(stream, handshake)
            }
            _ => self,
        }
    }

    fn handle_connect(stream: &mut TcpStream, handshake: HandshakeMessage) -> Self {
        match stream.peer_addr() {
            Ok(_) => {
                debug!("sending handshake message");
                match handshake.send(stream) {
                    Ok(_) => Self::Handshaking(handshake),
                    Err(err) => {
                        debug!(%err, "failed to send handshake message");
                        Self::Error
                    }
                }
            }
            Err(err) => {
                debug!(%err,"connection failed");
                Self::Error
            }
        }
    }

    fn handle_handshake(stream: &mut TcpStream, handshake: HandshakeMessage) -> Self {
        debug!("receiving remote handshake");
        match HandshakeMessage::receive(stream) {
            Ok(remote_handshake) => {
                if remote_handshake.info_hash == handshake.info_hash {
                    let remote_id = remote_handshake.peer_id;
                    debug!(%remote_id, "connected to peer");
                    Self::Connected(remote_id)
                } else {
                    debug!(
                        ?remote_handshake.info_hash,
                        "info_hash mismatch in received handshake"
                    );
                    Self::Error
                }
            }
            Err(err) => {
                debug!(%err, "handshake failed");
                Self::Error
            }
        }
    }
}
