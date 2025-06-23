use std::error::Error;

use bt_client::{AnnounceParams, make_announce_request, read_torrent_file};

fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents
        .get("announce")
        .and_then(|v| v.as_byte_string())
        .map(|v| v.to_string())
        .expect("Unable to retrieve announce URL");
    let info_hash = torrent_file_contents
        .get_dict_sha1("info")
        .expect("Unable to retrieve SHA-1 hash of `info` key")
        .clone();
    println!("\nYour announce url is: {}", announce_url);

    let announce_params = AnnounceParams {
        info_hash: info_hash,
        peer_id: vec![0x00; 20],
    };
    let response = make_announce_request(&announce_url, &announce_params)?;
    println!("Tracker response: {:?}", response);
    Ok(())
}
