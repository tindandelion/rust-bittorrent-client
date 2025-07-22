mod handshake_message;
mod peer_channel;
mod peer_messages;
mod piece_downloader;

use std::error::Error;

pub use peer_channel::PeerChannel;
pub use piece_downloader::DownloadChannel;
use piece_downloader::PieceDownloader;

use crate::types::Sha1;

pub fn download_file(
    channel: &mut impl DownloadChannel,
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut downloader = PieceDownloader::new(channel, piece_hashes, piece_length, file_length);
    let data = downloader.download_all()?;
    Ok(data)
}
