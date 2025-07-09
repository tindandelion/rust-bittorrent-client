use std::error::Error;

use bt_client::{
    AnnounceParams, FileDownloader, get_peer_list_from_response, make_announce_request,
    read_torrent_file,
    types::{Peer, PeerId, Sha1},
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents
        .get("announce")
        .and_then(|v| v.as_byte_string())
        .map(|v| v.to_string())
        .expect("Unable to retrieve announce URL");
    let info_hash = torrent_file_contents
        .get("info")
        .and_then(|v| v.as_dict())
        .map(|v| *v.sha1())
        .expect("Unable to retrieve SHA-1 hash of `info` key");

    println!("\nYour announce url is: {}", announce_url);

    let announce_params = AnnounceParams { info_hash, peer_id };
    let response = make_announce_request(&announce_url, &announce_params)?;
    let peers = get_peer_list_from_response(&response.as_bytes())?;
    println!("Total {} peers", peers.len());

    println!("Probing peers...");
    for peer in peers {
        print!("{}:{}\t-> ", peer.ip, peer.port);
        match probe_peer(&peer, info_hash, peer_id) {
            Ok(result) => println!("OK({})", result),
            Err(e) => println!("Err({})", e),
        }
    }

    Ok(())
}

fn probe_peer(peer: &Peer, info_hash: Sha1, peer_id: PeerId) -> Result<String, Box<dyn Error>> {
    let peer_addr = peer.to_socket_addr()?;
    let mut downloader = FileDownloader::connect(&peer_addr)?;
    let handshake_result = downloader.handshake(info_hash, peer_id)?;
    Ok(format!("{:?}", handshake_result))
}
