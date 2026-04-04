use std::{io, net::SocketAddr};

use tracing::instrument;

use crate::async_tcp::AsyncTcpStream;
use crate::downloader::peer_comm::{self, HandshakeMessage, PeerMessage};

use super::probe_result::ProbeResult;

// TODO: Check the received handshake info_hash
// TODO: Check for complete bitfield
#[instrument(skip(handshake))]
pub async fn connect_to_peer(addr: SocketAddr, handshake: HandshakeMessage) -> ProbeResult {
    let mut stream = init_connection(addr).await?;

    exchange_handshake(&mut stream, handshake).await?;
    read_bitfield(&mut stream).await?;
    request_interest(&mut stream).await?;

    Ok(stream.try_into()?)
}

#[instrument(skip(addr), err, ret(Display))]
async fn init_connection(addr: SocketAddr) -> io::Result<AsyncTcpStream> {
    AsyncTcpStream::connect(addr).await
}

#[instrument(skip(stream, my_handshake), err)]
async fn exchange_handshake<S>(
    stream: &mut S,
    my_handshake: HandshakeMessage,
) -> io::Result<HandshakeMessage>
where
    S: io::Write + peer_comm::AsyncReadExact,
{
    my_handshake.send(stream)?;
    HandshakeMessage::receive_async(stream).await
}

#[instrument(skip(stream), err, ret)]
async fn read_bitfield(stream: &mut AsyncTcpStream) -> io::Result<Vec<u8>> {
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
        types::{PeerId, Sha1},
    };

    use super::*;

    #[test]
    fn test_successful_handshake() {
        let mut stream = InMemoryStream::new();
        let handshake = HandshakeMessage::new(Sha1::random(), PeerId::random());

        stream.to_send.push(handshake.to_vec());
        poll_future(exchange_handshake(&mut stream, handshake)).unwrap();

        assert_eq!(vec![handshake.to_vec()], stream.received);
    }

    impl HandshakeMessage {
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

            let data = self.to_send.remove(0);
            buf.copy_from_slice(&data);
            Ok(())
        }
    }
}
