use std::error::Error;

use bt_client::{
    AnnounceParams, get_peer_list_from_response, make_announce_request, read_torrent_file,
    types::PeerId,
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

    Ok(())
}
