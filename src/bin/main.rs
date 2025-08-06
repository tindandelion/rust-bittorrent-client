use std::{error::Error, time::Duration};

use bt_client::{
    AnnounceParams,
    downloader::{self, PeerChannel},
    get_peer_list_from_response, get_piece_hashes, make_announce_request,
    probe_peers::probe_peers_sequential,
    read_torrent_file, request_complete_file,
    types::{PeerId, Sha1},
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents
        .get("announce")
        .and_then(|v| v.as_byte_string())
        .map(|v| v.to_string())
        .expect("Unable to retrieve announce URL");
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
    println!("\n* Your announce url is: {}", announce_url);

    let info_hash = *info.sha1();
    let announce_params = AnnounceParams { info_hash, peer_id };
    let response = make_announce_request(&announce_url, &announce_params)?;
    let peer_addrs = get_peer_list_from_response(response.as_bytes())?;
    println!("* Total {} peers", peer_addrs.len());

    println!("* Probing peers...");
    if let Some(mut channel) = probe_peers_sequential(&peer_addrs, |addr| {
        request_complete_file(addr, &info_hash, &peer_id, piece_hashes.len())
    }) {
        println!("* Connected to peer: {:?}", channel.peer_addr());

        let (file_content, download_duration) =
            download_file(&mut channel, piece_hashes, piece_length, file_length)?;
        println!(
            "* Received entire file, first 128 bytes: {}",
            hex::encode(&file_content[..128])
        );
        println!(
            "* File size: {}, download duration: {:?}",
            file_length, download_duration
        );
    } else {
        println!("* No peer responded");
    }

    Ok(())
}

fn download_file(
    channel: &mut PeerChannel,
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<(Vec<u8>, Duration), Box<dyn Error>> {
    println!("* Requesting file");
    let download_start = std::time::Instant::now();
    let file_content = downloader::download_file(channel, piece_hashes, piece_length, file_length)?;
    let download_duration = download_start.elapsed();
    Ok((file_content, download_duration))
}
