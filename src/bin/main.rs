use std::error::Error;

use bt_client::{
    AnnounceParams, download_file, get_peer_list_from_response, make_announce_request,
    probe_peers::probe_peers_sequential, request_complete_file, torrent::read_torrent_file,
    types::PeerId,
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent = read_torrent_file()?;
    let info = torrent.info;

    let info_hash = info.sha1();
    let piece_hashes = info.piece_hashes();

    let announce_params = AnnounceParams { info_hash, peer_id };
    let response = make_announce_request(&torrent.announce, &announce_params)?;
    let peer_addrs = get_peer_list_from_response(response.as_bytes())?;
    println!("* Total {} peers", peer_addrs.len());

    println!("* Probing peers...");
    if let Some(mut channel) = probe_peers_sequential(&peer_addrs, |addr| {
        request_complete_file(addr, &info_hash, &peer_id, piece_hashes.len())
    }) {
        println!("* Connected to peer: {:?}", channel.peer_addr());

        let (file_content, download_duration) =
            download_file(&mut channel, piece_hashes, info.piece_length, info.length)?;
        println!(
            "* Received entire file, first 128 bytes: {}",
            hex::encode(&file_content[..128])
        );
        println!(
            "* File size: {}, download duration: {:?}",
            info.length, download_duration
        );
    } else {
        println!("* No peer responded");
    }

    Ok(())
}
