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
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Handshaking(_) | Self::Error)
    }

    pub fn update(&self, stream: &mut impl PeerStream) -> ProbeUpdateResult {
        match self {
            Self::Connecting(context) => Self::handle_connect(stream, context),
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
