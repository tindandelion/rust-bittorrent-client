pub mod async_peer_connector;
mod async_tcp;
mod file_downloader;
pub mod peer_comm;
pub mod peer_connector;

use std::io;

pub use file_downloader::FileDownloader;
pub use peer_comm::PeerChannel;

use file_downloader::{Block, DownloadChannel, RequestChannel};
use peer_comm::PeerMessage;

impl RequestChannel for PeerChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        self.send(&PeerMessage::Request {
            piece_index,
            offset,
            length,
        })
    }
}

impl DownloadChannel for PeerChannel {
    fn receive(&mut self) -> io::Result<Block> {
        let msg = self.receive()?;
        match msg {
            PeerMessage::Piece {
                piece_index,
                offset,
                block,
            } => Ok(Block {
                piece_index,
                offset,
                data: block,
            }),
            other => Err(io::Error::other(format!(
                "Expected `piece` message, received: {:?}",
                other
            ))),
        }
    }
}
