use std::{
    io::{self},
    net::SocketAddr,
};

use tracing::{error, trace, warn};

use crate::{
    downloader::peer_comm::{HandshakeMessage, PeerMessage},
    types::{PeerId, Sha1},
};

pub trait PeerStream {
    fn peer_addr(&self) -> io::Result<SocketAddr>;
    fn send_handshake(&mut self, handshake: HandshakeMessage) -> io::Result<()>;
    fn receive_handshake(&mut self) -> io::Result<HandshakeMessage>;
    fn receive_message(&mut self) -> io::Result<PeerMessage>;
    fn send_message(&mut self, msg: PeerMessage) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeContext {
    pub peer_id: PeerId,
    pub info_hash: Sha1,
    pub piece_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProbeState {
    Connecting(ProbeContext),
    Handshaking(ProbeContext),
    WaitingForBitfield(usize, PeerId),
    Interested(PeerId),
    Unchoked(PeerId),
    Error,
}

#[derive(Debug)]
pub enum ProbeError {
    BitfieldSizeMismatch,
    IncompleteFile,
    InfoHashMismatch,
    UnexpectedPeerMessage,
    IO(io::Error),
}

pub type ProbeUpdateResult = Result<ProbeState, ProbeError>;

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

impl ProbeState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Unchoked(_))
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Unchoked(_) | Self::Error)
    }

    pub fn update(&self, stream: &mut impl PeerStream) -> ProbeUpdateResult {
        match self {
            Self::Connecting(context) => Self::handle_connect(stream, context),
            Self::Handshaking(context) => Self::handle_handshake(stream, context),
            Self::WaitingForBitfield(piece_count, peer_id) => {
                Self::handle_bitfield(stream, *piece_count, *peer_id)
            }
            Self::Interested(peer_id) => Self::handle_unchoke(stream, *peer_id),
            _ => Ok(self.clone()),
        }
    }

    fn handle_connect(stream: &mut impl PeerStream, context: &ProbeContext) -> ProbeUpdateResult {
        match stream.peer_addr() {
            Ok(_) => {
                trace!("sending handshake message");
                let handshake = HandshakeMessage::new(context.info_hash, context.peer_id);
                stream
                    .send_handshake(handshake)
                    .inspect_err(|err| error!(?err, "failed to send handshake message"))?;
                Ok(Self::Handshaking(*context))
            }
            Err(err) if err.kind() == io::ErrorKind::NotConnected => Ok(Self::Connecting(*context)),
            Err(err) => Err(err.into()),
        }
    }

    fn handle_handshake(stream: &mut impl PeerStream, context: &ProbeContext) -> ProbeUpdateResult {
        trace!("receiving remote handshake");
        let remote_handshake = stream.receive_handshake()?;
        if remote_handshake.info_hash != context.info_hash {
            warn!(
                ?remote_handshake.info_hash,
                "info_hash mismatch in received handshake"
            );
            return Err(ProbeError::InfoHashMismatch);
        }
        let remote_id = remote_handshake.peer_id;
        trace!(%remote_id, "connected to peer");
        Ok(Self::WaitingForBitfield(context.piece_count, remote_id))
    }

    fn handle_bitfield(
        stream: &mut impl PeerStream,
        piece_count: usize,
        peer_id: PeerId,
    ) -> ProbeUpdateResult {
        trace!("receiving bitfield");
        let msg = stream.receive_message()?;
        if let PeerMessage::Bitfield(bitfield) = msg {
            let expected_bitfield_size = piece_count.div_ceil(8);
            if bitfield.len() != expected_bitfield_size {
                return Err(ProbeError::BitfieldSizeMismatch);
            }
            if !is_bitfield_complete(&bitfield, piece_count) {
                return Err(ProbeError::IncompleteFile);
            }
            stream.send_message(PeerMessage::Interested)?;
            Ok(Self::Interested(peer_id))
        } else {
            error!(?msg, "unexpected message received");
            Err(ProbeError::UnexpectedPeerMessage)
        }
    }

    fn handle_unchoke(
        stream: &mut impl PeerStream,
        peer_id: PeerId,
    ) -> Result<ProbeState, ProbeError> {
        trace!("receiving unchoke message");
        let msg = stream.receive_message()?;
        if let PeerMessage::Unchoke = msg {
            trace!("unchoked, ready to download");
            Ok(Self::Unchoked(peer_id))
        } else {
            error!(?msg, "unexpected message received");
            Err(ProbeError::UnexpectedPeerMessage)
        }
    }
}

fn is_bitfield_complete(bitfield: &[u8], piece_count: usize) -> bool {
    for byte in &bitfield[..bitfield.len() - 1] {
        if *byte != 255 {
            return false;
        }
    }

    let mut pieces_in_last_byte = piece_count % 8;
    if pieces_in_last_byte == 0 {
        pieces_in_last_byte = 8;
    }
    let last_byte_mask = (128u8 as i8 >> (pieces_in_last_byte - 1)) as u8;
    let last_byte = bitfield[bitfield.len() - 1];
    if last_byte & last_byte_mask != last_byte_mask {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::{downloader::peer_comm::PeerMessage, types::Sha1};

    use super::*;

    mod connecting {
        use super::*;

        #[test]
        fn connect_to_remote_host_successfully() {
            let (state, context) = make_state();

            let mut stream = TestPeerStream::new();
            let next_state = state.update(&mut stream).unwrap();

            let expected_handshake = HandshakeMessage::new(context.info_hash, context.peer_id);
            assert_eq!(next_state, ProbeState::Handshaking(context));
            assert_eq!(stream.sent_handshake(), expected_handshake);
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

        fn make_state() -> (ProbeState, ProbeContext) {
            let context = ProbeContext {
                peer_id: PeerId::random(),
                info_hash: Sha1::random(),
                piece_count: 1,
            };
            let state = ProbeState::Connecting(context);
            (state, context)
        }
    }

    mod handshaking {
        use super::*;

        #[test]
        fn handshake_with_remote_host_successfully() {
            let (state, context) = make_state();
            let remote_handshake = HandshakeMessage::new(context.info_hash, PeerId::random());

            let mut stream = TestPeerStream::new();
            stream.set_remote_handshake(remote_handshake);
            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(
                next_state,
                ProbeState::WaitingForBitfield(context.piece_count, remote_handshake.peer_id)
            );
        }

        #[test]
        fn remote_handshake_has_different_info_hash() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            let remote_info_hash = Sha1::random();
            stream.set_remote_handshake(HandshakeMessage::new(remote_info_hash, PeerId::random()));
            let result = state.update(&mut stream);
            assert!(matches!(result, Err(ProbeError::InfoHashMismatch)));
        }

        fn make_state() -> (ProbeState, ProbeContext) {
            let context = ProbeContext {
                peer_id: PeerId::random(),
                info_hash: Sha1::random(),
                piece_count: 1,
            };
            let state = ProbeState::Handshaking(context);
            (state, context)
        }
    }

    mod waiting_for_bitfield {
        use crate::downloader::peer_comm::PeerMessage;

        use super::*;

        #[test]
        fn bitfield_received_successfully_sends_interested_message() {
            let (state, remote_peer_id) = make_state(16);
            let bitfield = vec![0b11111111, 0b11111111];

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(bitfield.clone()));

            let next_state = state.update(&mut stream).unwrap();

            assert_eq!(stream.sent_messages(), vec![PeerMessage::Interested]);
            assert_eq!(next_state, ProbeState::Interested(remote_peer_id));
        }

        #[test]
        fn error_when_error_receiving_message() {
            let (state, _) = make_state(1);

            let mut stream = TestPeerStream::new();
            let result = state.update(&mut stream);
            assert!(matches!(result, Err(ProbeError::IO(_))));
        }

        #[test]
        fn error_when_unexpected_message_received() {
            let (state, _) = make_state(1);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Unchoke);
            let result = state.update(&mut stream);

            assert!(matches!(result, Err(ProbeError::UnexpectedPeerMessage)));
        }

        #[test]
        fn error_when_bitfield_data_too_short() {
            let (state, _) = make_state(16);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(vec![0b11111111]));
            let result = state.update(&mut stream);

            assert!(matches!(result, Err(ProbeError::BitfieldSizeMismatch)));
        }

        #[test]
        fn error_when_bitfield_data_too_long() {
            let (state, _) = make_state(8);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(vec![0b11111111, 0b11111111]));
            let result = state.update(&mut stream);

            assert!(matches!(result, Err(ProbeError::BitfieldSizeMismatch)));
        }

        #[test]
        fn error_when_data_is_missing_intermediate_pieces() {
            let (state, _) = make_state(16);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(vec![0b10000000, 0b11111111]));
            let result = state.update(&mut stream);

            assert!(matches!(result, Err(ProbeError::IncompleteFile)));
        }

        #[test]
        fn error_when_data_missing_last_piece() {
            let (state, _) = make_state(15);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(vec![0b11111111, 0b11111100]));
            let result = state.update(&mut stream);

            assert!(matches!(result, Err(ProbeError::IncompleteFile)));
        }

        #[test]
        fn ignore_redundant_bits_in_last_byte() {
            let (state, _) = make_state(10);

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Bitfield(vec![0b11111111, 0b11001000]));
            let result = state.update(&mut stream);

            assert!(matches!(result, Ok(ProbeState::Interested(_))));
        }

        fn make_state(piece_count: usize) -> (ProbeState, PeerId) {
            let peer_id = PeerId::random();
            let state = ProbeState::WaitingForBitfield(piece_count, peer_id);
            (state, peer_id)
        }
    }

    mod interested {
        use super::*;

        #[test]
        fn receive_unchoke_message_successfully() {
            let (state, remote_peer_id) = make_state();

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Unchoke);

            let next_state = state.update(&mut stream).unwrap();
            assert_eq!(next_state, ProbeState::Unchoked(remote_peer_id));
        }

        #[test]
        fn error_when_unexpected_message_received() {
            let (state, _) = make_state();

            let mut stream = TestPeerStream::new();
            stream.remote_sends_message(PeerMessage::Interested);

            let result = state.update(&mut stream);
            assert!(matches!(result, Err(ProbeError::UnexpectedPeerMessage)));
        }

        fn make_state() -> (ProbeState, PeerId) {
            let peer_id = PeerId::random();
            let state = ProbeState::Interested(peer_id);
            (state, peer_id)
        }
    }

    struct TestPeerStream {
        peer_addr: String,
        sent_handshake: Option<HandshakeMessage>,
        remote_handshake: Option<HandshakeMessage>,
        message_from_remote: Option<PeerMessage>,
        sent_messages: Vec<PeerMessage>,
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
            self.message_from_remote
                .take()
                .ok_or(io::Error::new(io::ErrorKind::Other, "no message to send"))
        }

        fn send_message(&mut self, msg: PeerMessage) -> io::Result<()> {
            self.sent_messages.push(msg);
            Ok(())
        }
    }

    impl TestPeerStream {
        fn new() -> Self {
            Self {
                peer_addr: "127.0.0.1:12345".to_string(),
                sent_handshake: None,
                remote_handshake: None,
                message_from_remote: None,
                sent_messages: vec![],
            }
        }

        fn sent_handshake(&self) -> HandshakeMessage {
            self.sent_handshake.unwrap()
        }

        fn set_remote_handshake(&mut self, handshake: HandshakeMessage) {
            self.remote_handshake = Some(handshake);
        }

        fn remote_sends_message(&mut self, msg: PeerMessage) {
            self.message_from_remote = Some(msg);
        }

        fn sent_messages(&self) -> Vec<PeerMessage> {
            self.sent_messages.clone()
        }
    }
}
