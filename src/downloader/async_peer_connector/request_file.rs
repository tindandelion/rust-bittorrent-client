use std::{io, net::SocketAddr};

use tracing::instrument;

use crate::async_tcp::AsyncTcpStream;
use crate::downloader::PeerChannel;
use crate::downloader::peer_comm::{self, HandshakeMessage, PeerMessage};
use crate::types::{PeerId, Sha1};

use super::probe_result::{ProbeError, ProbeResult};

#[instrument(skip_all, fields(addr=%addr), err)]
pub async fn request_file_from_peer(
    addr: SocketAddr,
    info_hash: Sha1,
    peer_id: PeerId,
    piece_count: usize,
) -> ProbeResult<PeerChannel> {
    let mut stream = init_connection(addr).await?;

    let handshake = HandshakeMessage::new(info_hash, peer_id);
    let peer_id = exchange_handshake(&mut stream, handshake).await?;
    receive_bitfield(&mut stream, piece_count).await?;
    request_interest(&mut stream).await?;

    let std_stream: std::net::TcpStream = stream.try_into()?;
    let peer_channel = PeerChannel::from_stream(std_stream, peer_id)?;
    Ok(peer_channel)
}

#[instrument(skip(addr), err)]
async fn init_connection(addr: SocketAddr) -> io::Result<AsyncTcpStream> {
    AsyncTcpStream::connect(addr).await
}

#[instrument(skip(stream, my_handshake), err)]
async fn exchange_handshake<S>(
    stream: &mut S,
    my_handshake: HandshakeMessage,
) -> ProbeResult<PeerId>
where
    S: io::Write + peer_comm::AsyncReadExact,
{
    my_handshake.send(stream)?;
    let their_handshake = HandshakeMessage::receive_async(stream).await?;
    if their_handshake.info_hash != my_handshake.info_hash {
        return Err(ProbeError::InfoHashMismatch);
    }
    Ok(their_handshake.peer_id)
}

#[instrument(skip(stream), err)]
async fn receive_bitfield<S>(stream: &mut S, piece_count: usize) -> ProbeResult<()>
where
    S: peer_comm::AsyncReadExact,
{
    let msg = PeerMessage::receive_async(stream).await?;
    if let PeerMessage::Bitfield(bf) = msg {
        let expected_bitfield_size = piece_count.div_ceil(8);
        if bf.len() != expected_bitfield_size {
            return Err(ProbeError::BitfieldSizeMismatch);
        }
        if !is_bitfield_complete(&bf, piece_count) {
            return Err(ProbeError::IncompleteFile);
        }
        Ok(())
    } else {
        return Err(ProbeError::UnexpectedPeerMessage(msg));
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

#[instrument(skip(stream), err)]
async fn request_interest<S>(stream: &mut S) -> ProbeResult<()>
where
    S: io::Write + peer_comm::AsyncReadExact,
{
    PeerMessage::Interested.send(stream)?;

    let response = PeerMessage::receive_async(stream).await?;
    if matches!(response, PeerMessage::Unchoke) {
        Ok(())
    } else {
        Err(ProbeError::UnexpectedPeerMessage(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        async_tcp::test_helpers::poll_future,
        downloader::async_peer_connector::probe_result::ProbeError,
        types::{PeerId, Sha1},
    };

    use super::*;

    mod exchange_handshake {
        use super::*;

        #[test]
        fn test_successful_handshake() {
            let info_hash = Sha1::random();
            let their_peer_id = PeerId::random();
            let my_handshake = HandshakeMessage::new(info_hash, PeerId::random());
            let their_handshake = HandshakeMessage::new(info_hash, their_peer_id);

            let mut stream = InMemoryStream::new();
            stream.to_send.push(their_handshake.to_vec());

            let peer_id = poll_future(exchange_handshake(&mut stream, my_handshake)).unwrap();
            assert_eq!(their_peer_id, peer_id);
            assert_eq!(vec![my_handshake.to_vec()], stream.received);
        }

        #[test]
        fn test_error_when_received_mismatched_handshakes() {
            let my_handshake = HandshakeMessage::new(Sha1::random(), PeerId::random());
            let their_handshake = HandshakeMessage::new(Sha1::random(), PeerId::random());

            let mut stream = InMemoryStream::new();
            stream.to_send.push(their_handshake.to_vec());

            let err = poll_future(exchange_handshake(&mut stream, my_handshake))
                .expect_err("Expected an error");
            assert!(matches!(err, ProbeError::InfoHashMismatch));
        }
    }

    mod receive_bitfield {
        use super::*;

        #[test]
        fn receive_bitfield_successfully() {
            let bitfield = vec![0b11111111, 0b11111111];

            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            poll_future(receive_bitfield(&mut stream, 16)).unwrap();
        }

        #[test]
        fn error_when_received_unexpected_message() {
            let mut stream = InMemoryStream::new();
            stream.to_send.push(PeerMessage::Unchoke.to_vec());

            let err =
                poll_future(receive_bitfield(&mut stream, 16)).expect_err("Expected an error");
            assert!(matches!(
                err,
                ProbeError::UnexpectedPeerMessage(PeerMessage::Unchoke)
            ));
        }

        #[test]
        fn error_when_bitfield_data_too_short() {
            let bitfield = vec![0b11111111];
            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            let err =
                poll_future(receive_bitfield(&mut stream, 16)).expect_err("Expected an error");
            assert!(matches!(err, ProbeError::BitfieldSizeMismatch));
        }

        #[test]
        fn error_when_bitfield_data_too_long() {
            let bitfield = vec![0b11111111, 0b11111111];
            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            let err = poll_future(receive_bitfield(&mut stream, 8)).expect_err("Expected an error");
            assert!(matches!(err, ProbeError::BitfieldSizeMismatch));
        }

        #[test]
        fn error_when_data_is_missing_intermediate_pieces() {
            let bitfield = vec![0b10000000, 0b11111111];
            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            let err =
                poll_future(receive_bitfield(&mut stream, 16)).expect_err("Expected an error");
            assert!(matches!(err, ProbeError::IncompleteFile));
        }

        #[test]
        fn error_when_data_missing_last_piece() {
            let bitfield = vec![0b11111111, 0b11111100];
            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            let err =
                poll_future(receive_bitfield(&mut stream, 15)).expect_err("Expected an error");
            assert!(matches!(err, ProbeError::IncompleteFile));
        }

        #[test]
        fn ignore_redundant_bits_in_last_byte() {
            let bitfield = vec![0b11111111, 0b11001000];
            let mut stream = InMemoryStream::new();
            stream
                .to_send
                .push(PeerMessage::Bitfield(bitfield).to_vec());

            poll_future(receive_bitfield(&mut stream, 10)).unwrap();
        }
    }

    mod request_interest {
        use super::*;

        #[test]
        fn request_interest_successfully() {
            let mut stream = InMemoryStream::new();
            stream.to_send.push(PeerMessage::Unchoke.to_vec());

            poll_future(request_interest(&mut stream)).unwrap();
            assert_eq!(vec![PeerMessage::Interested.to_vec()], stream.received);
        }

        #[test]
        fn error_when_received_unexpected_message() {
            let mut stream = InMemoryStream::new();
            stream.to_send.push(PeerMessage::Interested.to_vec());

            let err = poll_future(request_interest(&mut stream)).expect_err("Expected an error");
            assert!(matches!(
                err,
                ProbeError::UnexpectedPeerMessage(PeerMessage::Interested)
            ));
        }
    }

    impl HandshakeMessage {
        pub fn to_vec(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            self.send(&mut vec).unwrap();
            vec
        }
    }

    impl PeerMessage {
        pub fn to_vec(&self) -> Vec<u8> {
            let mut vec = Vec::new();
            self.send(&mut vec).unwrap();
            vec
        }
    }

    struct InMemoryStream {
        received: Vec<Vec<u8>>,
        to_send: Vec<Vec<u8>>,
    }

    impl InMemoryStream {
        pub fn new() -> Self {
            Self {
                received: vec![],
                to_send: vec![],
            }
        }
    }

    impl io::Write for InMemoryStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.received.push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl peer_comm::AsyncReadExact for InMemoryStream {
        async fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
            if self.to_send.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "No data to send",
                ));
            }

            let data = self.to_send.first_mut().unwrap();
            buf.copy_from_slice(&data[..buf.len()]);
            data.drain(..buf.len());
            if data.is_empty() {
                self.to_send.remove(0);
            }

            Ok(())
        }
    }
}
