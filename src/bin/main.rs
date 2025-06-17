use std::error::Error;

use bt_client::{AnnounceParams, make_announce_request, read_torrent_file};

fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);

    let announce_params = AnnounceParams {
        info_hash: vec![42; 20],
        peer_id: vec![0x00; 20],
    };
    let response = make_announce_request(announce_url, &announce_params)?;
    println!("Tracker response: {:?}", response);
    Ok(())
}
