use std::time::Duration;

use bt_client::Torrent;
use bt_client::downloader::peer_connector::PeerConnector;
use bt_client::result::Result;
use bt_client::types::PeerId;

fn main() -> Result<()> {
    setup_tracing()?;

    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut successes = 0;

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let connector = PeerConnector::new(torrent.info.sha1, peer_id, torrent.info.pieces.len())
        .with_timeout(Duration::from_secs(30));

    for channel in connector.connect(addrs) {
        print!("{}\t\t\t", channel.peer_addr());
        println!("OK");
        successes += 1;
    }

    println!("\n\n ");
    println!("--- Successes: {successes}");

    Ok(())
}

fn setup_tracing() -> Result<()> {
    let log_filename = "request-file.log";

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(std::fs::File::create(&log_filename)?)
        .init();

    Ok(())
}
