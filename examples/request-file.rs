use std::collections::HashMap;
use std::time::Instant;

use bt_client::Torrent;
use bt_client::result::Result;
use bt_client::types::PeerId;

struct ErrorStats {
    count: usize,
    time_spent_ms: usize,
}

fn main() -> Result<()> {
    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut errors = HashMap::<String, ErrorStats>::new();

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let polling_start = Instant::now();
    for addr in addrs {
        print!("{addr}\t\t\t");
        let start = Instant::now();
        let result = torrent.request_file_from_address(addr, peer_id);
        let duration = start.elapsed().as_millis() as usize;
        match result {
            Ok(_) => println!("OK ({duration}ms)"),
            Err(e) => {
                let err_str = e.to_string();
                println!("Err({err_str}) ({duration}ms)");
                errors
                    .entry(err_str)
                    .and_modify(|info| {
                        info.count += 1;
                        info.time_spent_ms += duration;
                    })
                    .or_insert(ErrorStats {
                        count: 1,
                        time_spent_ms: duration,
                    });
            }
        }
    }
    let total_time = polling_start.elapsed().as_millis() as usize;

    let mut errors = errors.into_iter().collect::<Vec<_>>();
    errors.sort_by_key(|(_, info)| info.time_spent_ms);
    errors.reverse();

    println!(
        "\n\n --- Errors by time spent (total time {} ms):",
        total_time
    );
    for (index, (err, info)) in errors.iter().enumerate() {
        println!(
            "{index}:\t{err}: {} (total {} ms)",
            info.count, info.time_spent_ms
        );
    }

    Ok(())
}
