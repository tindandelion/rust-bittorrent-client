use std::error::Error;

use bt_client::read_torrent_file;
use reqwest;

fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);

    let response = reqwest::blocking::get(announce_url)?;
    println!("Response: {:?}", response.text()?);
    Ok(())
}
