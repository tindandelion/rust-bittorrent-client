mod handshake_message;
mod peer_channel;
mod peer_message;

use std::io;

pub use peer_channel::PeerChannel;
pub use peer_message::PeerMessage;

pub trait MessageChannel {
    fn receive(&mut self) -> io::Result<PeerMessage>;
    fn send(&mut self, msg: &PeerMessage) -> io::Result<()>;
}
