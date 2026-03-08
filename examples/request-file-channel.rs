use std::collections::HashMap;
use std::net::TcpStream;
use std::time::Instant;

use bt_client::ParPeerConnector;
use bt_client::Torrent;
use bt_client::downloader;
use bt_client::downloader::PeerChannel;
use bt_client::downloader::peer_connectors::ChannelConnector;
use bt_client::request_complete_file;
use bt_client::result::Result;
use bt_client::torrent::Info;
use bt_client::types::PeerId;
use tracing::Level;
use tracing::debug;
use tracing::instrument;

fn main() -> Result<()> {
    setup_tracing()?;

    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut errors = HashMap::<String, usize>::new();

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let connector = ChannelConnector::new(torrent.info.sha1, peer_id);

    for channel in connector.connect(addrs) {
        print!("{}\t\t\t", channel.peer_addr());

        let start = Instant::now();
        let result = request_complete_file(channel, torrent.info.pieces.len());
        let duration = start.elapsed().as_millis();
        match result {
            Ok(_) => println!("OK ({duration}ms)"),
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

    println!("\n\n --- Errors:");
    for (err, count) in errors {
        println!("{err}: {count}");
    }

    Ok(())
}

fn setup_tracing() -> Result<()> {
    let log_filename = "request-file-channel.log";

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::fs::File::create(&log_filename)?)
        .init();

    Ok(())
}
