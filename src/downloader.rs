mod file_downloader;
mod peer_comm;
pub mod peer_connectors;
pub mod request_download;

use std::io;

pub use file_downloader::FileDownloader;
pub use peer_comm::PeerChannel;
pub use request_download::request_complete_file;

use file_downloader::{Block, DownloadChannel, RequestChannel};
use peer_comm::{MessageChannel, PeerMessage};

impl<T: MessageChannel> RequestChannel for T {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
        self.send(&PeerMessage::Request {
            piece_index,
            offset,
            length,
        })
    }
}

impl<T: MessageChannel> DownloadChannel for T {
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
