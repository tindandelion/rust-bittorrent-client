mod file_downloader;
mod handshake_message;
mod peer_channel;
mod peer_comm;
mod request_download;

use std::error::Error;

use crate::types::Sha1;
use file_downloader::{DownloadChannel, FileDownloader, RequestChannel};
pub use peer_channel::PeerChannel;

pub fn download_file(
    channel: &mut (impl RequestChannel + DownloadChannel),
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    FileDownloader::new(channel, piece_hashes, piece_length, file_length)
        .download()
        .map_err(|e| e.into())
}

pub fn request_complete_file(
    channel: &mut PeerChannel,
    piece_count: usize,
) -> Result<(), Box<dyn Error>> {
    request_download::request_complete_file(channel, piece_count).map_err(|e| e.into())
}
