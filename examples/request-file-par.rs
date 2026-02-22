use std::collections::HashMap;
use std::time::Instant;

use bt_client::ParPeerConnector;
use bt_client::Torrent;
use bt_client::request_complete_file;
use bt_client::result::Result;
use bt_client::types::PeerId;

fn main() -> Result<()> {
    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut errors = HashMap::<String, usize>::new();

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let connector = ParPeerConnector::default();

    for stream in connector.connect(addrs) {
        print!(
            "{}\t\t\t",
            stream
                .peer_addr()
                .map(|addr| addr.to_string())
                .unwrap_or("unknown".to_string())
        );

        let start = Instant::now();
        let result = request_complete_file(stream, &peer_id, &torrent.info);
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
