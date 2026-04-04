use std::net::TcpStream;
use std::{io, net::SocketAddr};

use tracing::instrument;

use crate::async_tcp::AsyncTcpStream;
use crate::downloader::peer_comm::{self, HandshakeMessage, PeerMessage};
use crate::types::PeerId;

use super::probe_result::{ProbeError, ProbeResult};

// TODO: Check for complete bitfield
#[instrument(skip(handshake))]
pub async fn connect_to_peer(
    addr: SocketAddr,
    handshake: HandshakeMessage,
) -> ProbeResult<TcpStream> {
    let mut stream = init_connection(addr).await?;

    exchange_handshake(&mut stream, handshake).await?;
    receive_bitfield(&mut stream).await?;
    request_interest(&mut stream).await?;

    Ok(stream.try_into()?)
}

#[instrument(skip(addr), err, ret(Display))]
async fn init_connection(addr: SocketAddr) -> io::Result<AsyncTcpStream> {
    AsyncTcpStream::connect(addr).await
}

#[instrument(skip(stream, my_handshake), err(Debug))]
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

#[instrument(skip(stream), err, ret)]
async fn receive_bitfield<S>(stream: &mut S) -> io::Result<Vec<u8>>
where
    S: peer_comm::AsyncReadExact,
{
    let msg = PeerMessage::receive_async(stream).await?;
    if let PeerMessage::Bitfield(bf) = msg {
        Ok(bf)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("expected bitfield message, got {:?}", msg),
        ))
    }
}

#[instrument(skip(stream), err, ret)]
async fn request_interest(stream: &mut AsyncTcpStream) -> io::Result<()> {
    PeerMessage::Interested.send(stream)?;

    let msg = PeerMessage::receive_async(stream).await?;
    if let PeerMessage::Unchoke = msg {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("expected unchoke message, got {:?}", msg),
        ))
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

    #[test]
    fn test_successful_handshake() {
        let info_hash = Sha1::random();
        let my_handshake = HandshakeMessage::new(info_hash, PeerId::random());
        let their_handshake = HandshakeMessage::new(info_hash, PeerId::random());

        let mut stream = InMemoryStream::new();
        stream.to_send.push(their_handshake.to_vec());

        poll_future(exchange_handshake(&mut stream, my_handshake)).unwrap();
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

    #[test]
    fn test_received_bitfield_successfully() {
        let bitfield = vec![0b11111111, 0b11111111];

        let mut stream = InMemoryStream::new();
        stream
            .to_send
            .push(PeerMessage::Bitfield(bitfield).to_vec());

        poll_future(receive_bitfield(&mut stream)).unwrap();
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
