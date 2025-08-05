use std::{
    error::Error,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use bt_client::{
    downloader::{self, PeerChannel},
    get_piece_hashes, read_torrent_file, request_complete_file,
    types::{PeerId, Sha1},
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent_file_contents = read_torrent_file();
    let info = torrent_file_contents
        .get("info")
        .and_then(|v| v.as_dict())
        .expect("Unable to retrieve `info` key");
    let piece_hashes = info
        .get("pieces")
        .and_then(|v| v.as_byte_string())
        .map(get_piece_hashes)
        .expect("Unable to retrieve `pieces` key");
    let piece_length = info
        .get("piece length")
        .and_then(|v| v.as_int())
        .map(|v| *v as u32)
        .expect("Unable to retrieve `piece length` key");
    let file_length = info
        .get("length")
        .and_then(|v| v.as_int())
        .map(|v| *v as usize)
        .expect("Unable to retrieve `length` key");

    println!(
        "* Total pieces {}, piece length {}",
        piece_hashes.len(),
        piece_length
    );

    let info_hash = *info.sha1();
    let mut local_peer =
        request_complete_file(&local_address(), &info_hash, &peer_id, piece_hashes.len())?;
    println!("* Connected to local peer: {:?}", local_peer.peer_addr());

    let (file_content, download_duration) =
        download_file(&mut local_peer, piece_hashes, piece_length, file_length)?;
    println!(
        "* Received entire file, first 128 bytes: {}",
        hex::encode(&file_content[..128])
    );
    println!(
        "* File size: {}, download duration: {:?}",
        file_length, download_duration
    );

    Ok(())
}

fn download_file(
    channel: &mut PeerChannel,
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<(Vec<u8>, Duration), Box<dyn Error>> {
    println!("* Unchoked, requesting file");
    let download_start = std::time::Instant::now();
    let file_content = downloader::download_file(channel, piece_hashes, piece_length, file_length)?;
    let download_duration = download_start.elapsed();
    Ok((file_content, download_duration))
}

fn local_address() -> SocketAddr {
    "127.0.0.1:26408".to_socket_addrs().unwrap().next().unwrap()
}
