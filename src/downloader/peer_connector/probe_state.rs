use std::{
    io::{self, Read, Write},
    net::SocketAddr,
};

use tracing::debug;

use crate::{downloader::peer_comm::handshake_message::HandshakeMessage, types::PeerId};

pub trait PeerStream: Read + Write {
    fn peer_addr(&self) -> io::Result<SocketAddr>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

    pub fn handle_event(self, stream: &mut impl PeerStream, is_readable: bool) -> Self {
        match self {
            Self::Connecting(handshake) => Self::handle_connect(stream, handshake),
            Self::Handshaking(handshake) if is_readable => {
                Self::handle_handshake(stream, handshake)
            }
            _ => self,
        }
    }

    fn handle_connect(stream: &mut impl PeerStream, handshake: HandshakeMessage) -> Self {
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
            Err(err) if err.kind() == io::ErrorKind::NotConnected => Self::Connecting(handshake),
            Err(err) => {
                debug!(%err,"connection failed");
                Self::Error
            }
        }
    }

    fn handle_handshake(stream: &mut impl PeerStream, handshake: HandshakeMessage) -> Self {
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

#[cfg(test)]
mod tests {
    use crate::types::Sha1;

    use super::*;

    mod connecting {
        use super::*;

        #[test]
        fn connect_to_remote_host_successfully() {
            let (state, handshake) = make_state();

            let mut stream = TestPeerStream::new();
            let next_state = state.handle_event(&mut stream, false);

            assert_eq!(next_state, ProbeState::Handshaking(handshake));
            assert_eq!(stream.sent_handshake(), handshake);
        }

        #[test]
        fn connection_to_remote_host_in_progress() {
            let (state, handshake) = make_state();

            let mut stream = TestPeerStream::new();
            stream.peer_addr = "not-connected".to_string();
            let next_state = state.handle_event(&mut stream, false);

            assert_eq!(next_state, ProbeState::Connecting(handshake));
        }

        #[test]
        fn connection_to_remote_host_failed() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.peer_addr = "error".to_string();
            let next_state = state.handle_event(&mut stream, false);

            assert_eq!(next_state, ProbeState::Error);
        }

        fn make_state() -> (ProbeState, HandshakeMessage) {
            let handshake = HandshakeMessage::new(Sha1::random(), PeerId::random());
            let state = ProbeState::Connecting(handshake);
            (state, handshake)
        }
    }

    mod handshaking {
        use super::*;

        #[test]
        fn handshake_with_remote_host_successfully() {
            let (state, info_hash) = make_state();
            let remote_handshake = HandshakeMessage::new(info_hash, PeerId::random());

            let mut stream = TestPeerStream::new();
            stream.set_remote_handshake(remote_handshake);
            let next_state = state.handle_event(&mut stream, true);

            assert_eq!(next_state, ProbeState::Connected(remote_handshake.peer_id));
        }

        #[test]
        fn remote_handshake_has_different_info_hash() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.set_remote_handshake(HandshakeMessage::new(Sha1::random(), PeerId::random()));
            let next_state = state.handle_event(&mut stream, true);

            assert_eq!(next_state, ProbeState::Error);
        }

        #[test]
        fn remote_handshake_is_invalid() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.data_to_send = vec![0x01; 68];
            let next_state = state.handle_event(&mut stream, true);

            assert_eq!(next_state, ProbeState::Error);
        }

        fn make_state() -> (ProbeState, Sha1) {
            let info_hash = Sha1::random();
            let handshake = HandshakeMessage::new(info_hash, PeerId::random());
            let state = ProbeState::Handshaking(handshake);
            (state, info_hash)
        }
    }

    struct TestPeerStream {
        peer_addr: String,
        received_data: Vec<u8>,
        data_to_send: Vec<u8>,
    }

    impl PeerStream for TestPeerStream {
        fn peer_addr(&self) -> io::Result<SocketAddr> {
            if self.peer_addr == "not-connected" {
                Err(io::Error::new(io::ErrorKind::NotConnected, "not connected"))
            } else if self.peer_addr == "error" {
                Err(io::Error::new(io::ErrorKind::Other, "error"))
            } else {
                Ok("127.0.0.1:12345".parse().unwrap())
            }
        }
    }

    impl TestPeerStream {
        fn new() -> Self {
            Self {
                peer_addr: "127.0.0.1:12345".to_string(),
                received_data: Vec::new(),
                data_to_send: Vec::new(),
            }
        }

        fn sent_handshake(&self) -> HandshakeMessage {
            HandshakeMessage::receive(&mut self.received_data.as_slice()).unwrap()
        }

        fn set_remote_handshake(&mut self, handshake: HandshakeMessage) {
            handshake.send(&mut self.data_to_send).unwrap();
        }
    }

    impl Read for TestPeerStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            buf.copy_from_slice(&self.data_to_send);
            Ok(self.data_to_send.len())
        }
    }

    impl Write for TestPeerStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.received_data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
