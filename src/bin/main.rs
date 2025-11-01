use std::error::Error;

use bt_client::{
    download_file, probe_peers::probe_peers_sequential, request_complete_file,
    torrent::read_torrent_file, tracker::AnnounceRequest, types::PeerId,
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent = read_torrent_file()?;
    let info = torrent.info;

    let announce_request = AnnounceRequest {
        tracker_url: torrent.announce,
        info_hash: info.sha1,
        peer_id,
    };
    let peer_addrs = announce_request.fetch_peer_addresses()?;
    println!("* Total {} peers", peer_addrs.len());

    println!("* Probing peers...");
    if let Some(mut channel) = probe_peers_sequential(&peer_addrs, |addr| {
        request_complete_file(addr, &info.sha1, &peer_id, info.pieces.len())
    }) {
        println!("* Connected to peer: {:?}", channel.peer_addr());

        let (file_content, download_duration) =
            download_file(&mut channel, info.pieces, info.piece_length, info.length)?;
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
