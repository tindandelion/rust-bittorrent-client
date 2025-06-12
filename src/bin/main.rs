use bt_client::read_torrent_file;

fn main() {
    let torrent_contents = read_torrent_file();
    let announce_url = torrent_contents.get_string("announce").unwrap();
    println!("Your announce url: {}", announce_url);
}
