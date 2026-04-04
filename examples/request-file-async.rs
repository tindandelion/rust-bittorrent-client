use std::time::Duration;

use bt_client::Torrent;
use bt_client::downloader::async_peer_connector::PeerConnector;
use bt_client::result::Result;
use bt_client::types::PeerId;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    setup_tracing()?;

    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut successes = 0;

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let connector = PeerConnector::new(torrent.info.sha1, peer_id, torrent.info.pieces.len())
        .with_timeout(Duration::from_secs(10));

    println!("Connecting to {} peers", addrs.len());

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
    let log_filename = "request-file-async.log";

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("bt_client=trace".parse().unwrap()),
        )
        .with_writer(std::fs::File::create(&log_filename)?)
        .init();

    Ok(())
}
