use std::time::{Duration, Instant};

use bt_client::Torrent;
use bt_client::downloader::peer_connector::PeerConnector;
use bt_client::result::Result;
use bt_client::types::PeerId;
use rand::seq::SliceRandom;

const NUM_EXPERIMENTS: usize = 10;

fn main() -> Result<()> {
    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut ttds: Vec<Duration> = Vec::new();

    let addrs = torrent.fetch_peer_addresses(peer_id)?;
    let mut rng = rand::rng();

    println!("Measuring Time to Unchoke (TTU)\n\n");

    for i in 0..NUM_EXPERIMENTS {
        println!("Experiment {} of {NUM_EXPERIMENTS}", i + 1);
        let connector = PeerConnector::new(torrent.info.sha1, peer_id, torrent.info.pieces.len())
            .with_timeout(Duration::from_secs(30));

        let mut shuffled_addrs = addrs.clone();
        shuffled_addrs.shuffle(&mut rng);

        let start = Instant::now();
        let _ = connector.connect(shuffled_addrs).next().unwrap();
        let ttd = start.elapsed();
        ttds.push(ttd);
    }

    let avg_ttd = ttds.iter().sum::<Duration>() / NUM_EXPERIMENTS as u32;
    let min_ttd = ttds.iter().min().unwrap();
    let max_ttd = ttds.iter().max().unwrap();

    println!("\n\n--------------------------------");
    println!("Average: {}ms", avg_ttd.as_millis());
    println!("Min: {}ms", min_ttd.as_millis());
    println!("Max: {}ms", max_ttd.as_millis());
    Ok(())
}
