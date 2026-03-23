use std::collections::HashMap;
use std::time::{Duration, Instant};

use bt_client::Torrent;
use bt_client::downloader::peer_connector::PeerConnector;
use bt_client::request_complete_file;
use bt_client::result::Result;
use bt_client::types::PeerId;

fn main() -> Result<()> {
    setup_tracing()?;

    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut errors = HashMap::<String, usize>::new();
    let mut successes = 0;

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let connector = PeerConnector::new(torrent.info.sha1, peer_id, torrent.info.pieces.len())
        .with_timeout(Duration::from_secs(10));

    for channel in connector.connect(addrs) {
        print!("{}\t\t\t", channel.peer_addr());

        let start = Instant::now();
        let result = request_complete_file(channel, torrent.info.pieces.len());
        let duration = start.elapsed().as_millis();
        match result {
            Ok(_) => {
                println!("OK ({duration}ms)");
                successes += 1;
            }
            Err(e) => {
                let err_str = e.to_string();
                println!("Err({err_str}) ({duration}ms)");
                errors
                    .entry(err_str)
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }
    }

    println!("\n\n ");
    println!("--- Successes: {successes}");
    println!("--- Errors:");
    for (err, count) in errors {
        println!("{err}: {count}");
    }

    Ok(())
}

fn setup_tracing() -> Result<()> {
    let log_filename = "request-file.log";

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::fs::File::create(&log_filename)?)
        .init();

    Ok(())
}
