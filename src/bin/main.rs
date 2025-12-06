use std::error::Error;

use bt_client::{
    download_file, probe_peers::probe_peers_sequential, request_complete_file, torrent::Torrent,
    tracker::AnnounceRequest, types::PeerId,
};
use tracing::{Level, error, info};

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let torrent = Torrent::read_default_file()?;
    let info = torrent.info;

    let peer_id = PeerId::default();
    let announce_request = AnnounceRequest {
        tracker_url: torrent.announce,
        info_hash: info.sha1,
        peer_id,
    };
    let peer_addrs = announce_request.fetch_peer_addresses()?;
    info!(peer_count = peer_addrs.len(), "Received peer addresses");

    info!("Probing peers");
    if let Some(mut channel) = probe_peers_sequential(&peer_addrs, |addr| {
        request_complete_file(addr, &info.sha1, &peer_id, info.pieces.len())
    }) {
        info!(
            file_size = info.length,
            piece_count = info.pieces.len(),
            peer_address = %channel.peer_addr(),
            remote_id = %channel.remote_id(),
            "Downloading file"
        );
        let (file_content, download_duration) =
            download_file(&mut channel, info.pieces, info.piece_length, info.length)?;
        info!(
            file_bytes = hex::encode(&file_content[..128]),
            file_size = info.length,
            download_duration = format!("{:.2?}", download_duration),
            "Received entire file"
        );
    } else {
        error!("No peer responded");
    }

    Ok(())
}
