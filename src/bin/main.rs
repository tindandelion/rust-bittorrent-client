use std::error::Error;

use bt_client::{TrackerRequestParams, make_tracker_request_url, read_torrent_file};
use reqwest;

fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);

    let request_params = TrackerRequestParams {
        info_hash: vec![
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
            0xef, 0x12, 0x34, 0x56, 0x78, 0x9a,
        ],
        peer_id: vec![0x00; 20],
    };

    let url = make_tracker_request_url(announce_url, &request_params)?;

    let response = reqwest::blocking::get(url)?;
    println!("Response: {:?}", response.text()?);
    Ok(())
}
