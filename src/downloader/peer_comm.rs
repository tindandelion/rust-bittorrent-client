mod handshake_message;
mod peer_channel;
mod peer_message;

pub use handshake_message::HandshakeMessage;
pub use peer_channel::PeerChannel;
pub use peer_message::PeerMessage;

use crate::async_tcp::AsyncTcpStream;

pub trait AsyncReadExact {
    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = std::io::Result<()>>;
}

impl AsyncReadExact for AsyncTcpStream {
    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = std::io::Result<()>> {
        AsyncTcpStream::read_exact(self, buf)
    }
}
