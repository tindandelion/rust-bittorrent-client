mod downloader;
mod probe_peers;
pub mod ratatui_ui;
mod torrent;
mod tracker;
mod types;
mod util;

use tracing::{Level, debug, error, info, instrument};

use crate::{
    downloader::PeerChannel, probe_peers::probe_peers_sequential, torrent::Info,
    tracker::AnnounceRequest, types::PeerId,
};
use std::net::SocketAddr;
pub use torrent::Torrent;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Torrent {
    pub fn download(self) -> Result<()> {
        let info = self.info;

        let peer_id = PeerId::default();
        let announce_request = AnnounceRequest {
            tracker_url: self.announce,
            info_hash: info.sha1,
            peer_id,
        };
        let peer_addrs = announce_request.fetch_peer_addresses()?;
        info!(peer_count = peer_addrs.len(), "Received peer addresses");

        info!("Probing peers");
        if let Some(mut channel) = probe_peers_sequential(&peer_addrs, |addr| {
            request_complete_file(addr, &peer_id, &info)
        }) {
            info!(
                file_size = info.length,
                piece_count = info.pieces.len(),
                peer_address = %channel.peer_addr(),
                remote_id = %channel.remote_id(),
                "Downloading file"
            );
            let (file_content, download_duration) = util::elapsed(|| {
                downloader::FileDownloader::new(
                    &mut channel,
                    info.pieces,
                    info.piece_length,
                    info.length,
                )
                .download()
            })?;
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
}

#[instrument(skip(info, peer_id), level = Level::DEBUG)]
fn request_complete_file(
    peer_addr: &SocketAddr,
    peer_id: &PeerId,
    info: &Info,
) -> Result<PeerChannel> {
    debug!("Connecting to peer");
    let mut channel = PeerChannel::connect(peer_addr, &info.sha1, peer_id)
        .inspect(|channel| debug!(remote_id = %channel.remote_id(), "Connected"))
        .inspect_err(|error| debug!(%error, "Failed to connect"))?;

    debug!("Connected, requesting file");
    downloader::request_complete_file(&mut channel, info.pieces.len())?;
    debug!("Ready to download");
    Ok(channel)
}
