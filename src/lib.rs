pub mod downloader;
pub mod probe_peers;
pub mod torrent;
pub mod tracker;
pub mod types;

use crate::{
    downloader::PeerChannel,
    types::{PeerId, Sha1},
};
use std::net::SocketAddr;

pub fn request_complete_file(
    peer_addr: &SocketAddr,
    info_hash: &Sha1,
    peer_id: &PeerId,
    piece_count: usize,
) -> Result<PeerChannel, Box<dyn std::error::Error>> {
    eprint!("{:?}\t-> ", peer_addr);
    let mut channel = match PeerChannel::connect(peer_addr, info_hash, peer_id) {
        Ok(channel) => {
            eprintln!("OK({})", channel.remote_id());
            Ok(channel)
        }
        Err(e) => {
            eprintln!("Err({})", e);
            Err(e)
        }
    }?;

    eprintln!("* Connected, requesting file");
    downloader::request_complete_file(&mut channel, piece_count)?;
    eprintln!("* Ready to download");
    Ok(channel)
}

pub fn download_file(
    channel: &mut PeerChannel,
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<(Vec<u8>, std::time::Duration), Box<dyn std::error::Error>> {
    eprintln!("* Downloading file");
    let download_start = std::time::Instant::now();
    let file_content = downloader::download_file(channel, piece_hashes, piece_length, file_length)?;
    let download_duration = download_start.elapsed();
    Ok((file_content, download_duration))
}
