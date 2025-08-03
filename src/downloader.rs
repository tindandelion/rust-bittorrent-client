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
    FileDownloader::new(channel, piece_hashes, piece_length, file_length)
        .download()
        .map_err(|e| e.into())
}

pub fn request_complete_file(channel: &mut PeerChannel) -> Result<(), Box<dyn Error>> {
    let bitfield = channel.receive_bitfield()?;
    println!("* Received bitfield: {}", hex::encode(bitfield));

    println!("* Sending `interested` message");
    channel.send_interested()?;

    println!("* Receiving `unchoke` message");
    channel.receive_unchoke()?;

    Ok(())
}
