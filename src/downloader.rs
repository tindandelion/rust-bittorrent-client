mod file_downloader;
mod handshake_message;
mod peer_channel;
mod peer_messages;

use std::error::Error;

pub use file_downloader::DownloadChannel;
use file_downloader::FileDownloader;
pub use peer_channel::PeerChannel;

use crate::{downloader::file_downloader::RequestChannel, types::Sha1};

pub fn download_file(
    channel: &mut (impl RequestChannel + DownloadChannel),
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let data = FileDownloader::new(channel, piece_hashes, piece_length, file_length).download()?;
    Ok(data)
}
