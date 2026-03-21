use std::{
    io::{self},
    net::SocketAddr,
};

use tracing::{error, trace};

use crate::{
    downloader::peer_comm::{PeerMessage, handshake_message::HandshakeMessage},
    types::PeerId,
};

pub trait PeerStream {
    fn peer_addr(&self) -> io::Result<SocketAddr>;
    fn send_handshake(&mut self, handshake: HandshakeMessage) -> io::Result<()>;
    fn receive_handshake(&mut self) -> io::Result<HandshakeMessage>;
    fn receive_message(&mut self) -> io::Result<PeerMessage>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProbeState {
    Connecting(HandshakeMessage),
    Handshaking(HandshakeMessage),
    WaitingForBitfield(PeerId),
    BitfieldReceived(PeerId, Vec<u8>),
    Error,
}

impl ProbeState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::BitfieldReceived(_, _))
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::BitfieldReceived(_, _) | Self::Error)
    }

    pub fn update(&self, stream: &mut impl PeerStream) -> io::Result<Self> {
        match self {
            Self::Connecting(handshake) => Self::handle_connect(stream, *handshake),
            Self::Handshaking(handshake) => Self::handle_handshake(stream, *handshake),
            Self::WaitingForBitfield(peer_id) => Self::handle_bitfield(stream, *peer_id),
            _ => Ok(self.clone()),
        }
    }

    fn handle_connect(
        stream: &mut impl PeerStream,
        handshake: HandshakeMessage,
    ) -> io::Result<Self> {
        match stream.peer_addr() {
            Ok(_) => {
                trace!("sending handshake message");
                stream
                    .send_handshake(handshake)
                    .inspect_err(|err| error!(?err, "failed to send handshake message"))?;
                Ok(Self::Handshaking(handshake))
            }
            Err(err) if err.kind() == io::ErrorKind::NotConnected => {
                Ok(Self::Connecting(handshake))
            }
            Err(err) => Err(err),
        }
    }

    fn handle_handshake(
        stream: &mut impl PeerStream,
        handshake: HandshakeMessage,
    ) -> io::Result<Self> {
        trace!("receiving remote handshake");
        let remote_handshake = stream.receive_handshake()?;
        if remote_handshake.info_hash != handshake.info_hash {
            error!(
                ?remote_handshake.info_hash,
                "info_hash mismatch in received handshake"
            );
            return Ok(Self::Error);
        }
        let remote_id = remote_handshake.peer_id;
        trace!(%remote_id, "connected to peer");
        Ok(Self::WaitingForBitfield(remote_id))
    }

    fn handle_bitfield(stream: &mut impl PeerStream, peer_id: PeerId) -> io::Result<Self> {
        trace!("receiving bitfield");
        let msg = stream.receive_message()?;
        if let PeerMessage::Bitfield(bitfield) = msg {
            Ok(Self::BitfieldReceived(peer_id, bitfield))
        } else {
            error!(?msg, "unexpected message received");
            Ok(Self::Error)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{downloader::peer_comm::PeerMessage, types::Sha1};

    use super::*;

    mod connecting {
        use super::*;

        #[test]
        fn connect_to_remote_host_successfully() {
            let (state, handshake) = make_state();

            let mut stream = TestPeerStream::new();
            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(next_state, ProbeState::Handshaking(handshake));
            assert_eq!(stream.sent_handshake(), handshake);
        }

        #[test]
        fn connection_to_remote_host_in_progress() {
            let (state, handshake) = make_state();

            let mut stream = TestPeerStream::new();
            stream.peer_addr = "not-connected".to_string();
            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(next_state, ProbeState::Connecting(handshake));
        }

        #[test]
        fn connection_to_remote_host_failed() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.peer_addr = "error".to_string();
            let _ = state.update(&mut stream).expect_err("expected an error");
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
            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(
                next_state,
                ProbeState::WaitingForBitfield(remote_handshake.peer_id)
            );
        }

        #[test]
        fn remote_handshake_has_different_info_hash() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.set_remote_handshake(HandshakeMessage::new(Sha1::random(), PeerId::random()));
            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(next_state, ProbeState::Error);
        }

        fn make_state() -> (ProbeState, Sha1) {
            let info_hash = Sha1::random();
            let handshake = HandshakeMessage::new(info_hash, PeerId::random());
            let state = ProbeState::Handshaking(handshake);
            (state, info_hash)
        }
    }

    mod waiting_for_bitfield {
        use crate::downloader::peer_comm::PeerMessage;

        use super::*;

        #[test]
        fn bitfield_received_successfully() {
            let (state, peer_id) = make_state();
            let bitfield = vec![0b11111111, 0b11111111];

            let mut stream = TestPeerStream::new();
            stream.sends_peer_message(PeerMessage::Bitfield(bitfield.clone()));

            let next_state = state.update(&mut stream).unwrap();
            assert_eq!(next_state, ProbeState::BitfieldReceived(peer_id, bitfield));
        }

        #[test]
        fn error_when_error_receiving_message() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            let _ = state.update(&mut stream).expect_err("expected an error");
        }

        fn make_state() -> (ProbeState, PeerId) {
            let peer_id = PeerId::random();
            let state = ProbeState::WaitingForBitfield(peer_id);
            (state, peer_id)
        }
    }

    struct TestPeerStream {
        peer_addr: String,
        sent_handshake: Option<HandshakeMessage>,
        remote_handshake: Option<HandshakeMessage>,
        message_to_send: Option<PeerMessage>,
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

        fn send_handshake(&mut self, handshake: HandshakeMessage) -> io::Result<()> {
            self.sent_handshake = Some(handshake);
            Ok(())
        }

        fn receive_handshake(&mut self) -> io::Result<HandshakeMessage> {
            self.remote_handshake.ok_or(io::Error::new(
                io::ErrorKind::Other,
                "no remote handshake to send",
            ))
        }

        fn receive_message(&mut self) -> io::Result<PeerMessage> {
            self.message_to_send
                .take()
                .ok_or(io::Error::new(io::ErrorKind::Other, "no message to send"))
        }
    }

    impl TestPeerStream {
        fn new() -> Self {
            Self {
                peer_addr: "127.0.0.1:12345".to_string(),
                sent_handshake: None,
                remote_handshake: None,
                message_to_send: None,
            }
        }

        fn sent_handshake(&self) -> HandshakeMessage {
            self.sent_handshake.unwrap()
        }

        fn set_remote_handshake(&mut self, handshake: HandshakeMessage) {
            self.remote_handshake = Some(handshake);
        }

        fn sends_peer_message(&mut self, msg: PeerMessage) {
            self.message_to_send = Some(msg);
        }
    }
}
