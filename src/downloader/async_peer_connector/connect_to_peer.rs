use std::{io, net::SocketAddr};

use tracing::instrument;

use crate::async_tcp::AsyncTcpStream;
use crate::downloader::peer_comm::{HandshakeMessage, PeerMessage};

// TODO: Check the received handshake info_hash
#[instrument(skip(handshake))]
pub async fn connect_to_peer(
    addr: SocketAddr,
    handshake: HandshakeMessage,
) -> io::Result<std::net::TcpStream> {
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
async fn exchange_handshake(
    stream: &mut AsyncTcpStream,
    my_handshake: HandshakeMessage,
) -> io::Result<HandshakeMessage> {
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
