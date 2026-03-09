use std::collections::HashMap;
use std::net::TcpStream;
use std::time::Instant;

use bt_client::ParPeerConnector;
use bt_client::Torrent;
use bt_client::downloader;
use bt_client::downloader::PeerChannel;
use bt_client::downloader::peer_comm::handshake_message::HandshakeMessage;
use bt_client::result::Result;
use bt_client::torrent::Info;
use bt_client::types::PeerId;
use bt_client::types::Sha1;
use tracing::Level;
use tracing::debug;
use tracing::instrument;

fn main() -> Result<()> {
    setup_tracing()?;

    let torrent = Torrent::read_default_file()?;
    let peer_id = PeerId::default();
    let mut errors = HashMap::<String, usize>::new();
    let mut successes = 0;

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
        let result = request_complete_file(stream, peer_id, &torrent.info);
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
    let log_filename = "request-file-par.log";

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::fs::File::create(&log_filename)?)
        .init();

    Ok(())
}

#[instrument(skip_all, err(level = Level::WARN), level = Level::DEBUG)]
fn request_complete_file(
    mut stream: TcpStream,
    peer_id: PeerId,
    info: &Info,
) -> Result<PeerChannel> {
    let remote_id = exchange_handshake(&mut stream, info.sha1, peer_id)?;
    let mut channel = PeerChannel::from_stream(stream, remote_id)
        .inspect(|channel| debug!(remote_id = %channel.remote_id(), "Connected"))?;

    debug!("Connected, requesting file");
    downloader::request_complete_file(&mut channel, info.pieces.len())?;
    debug!("Ready to download");
    Ok(channel)
}

#[instrument(skip_all, err(level=Level::WARN), level = Level::DEBUG)]
pub fn exchange_handshake(
    stream: &mut TcpStream,
    info_hash: Sha1,
    peer_id: PeerId,
) -> std::io::Result<PeerId> {
    HandshakeMessage::new(info_hash, peer_id).send(stream)?;
    HandshakeMessage::receive(stream).map(|msg| msg.peer_id)
}
