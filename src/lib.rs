mod downloader;
mod probe_peers;
pub mod ratatui_ui;
pub mod result;
mod torrent;
mod tracker;
pub mod types;
mod util;

use tracing::{Level, debug, error, info, instrument};

use crate::{
    downloader::PeerChannel, probe_peers::probe_peers_sequential, ratatui_ui::AppEvent,
    torrent::Info, tracker::AnnounceRequest, types::PeerId,
};
use result::Result;
use std::{net::SocketAddr, sync::mpsc::Sender, time::Duration};
pub use torrent::Torrent;

#[derive(Debug)]
pub struct DownloadedFile {
    pub content: Vec<u8>,
    pub download_duration: Duration,
}

impl Torrent {
    pub fn download(self, event_sender: &Sender<AppEvent>) -> Result<()> {
        let peer_id = PeerId::default();
        let announce_request = AnnounceRequest {
            tracker_url: self.announce.clone(),
            info_hash: self.info.sha1,
            peer_id,
        };
        let peer_addrs = announce_request.fetch_peer_addresses()?;
        info!(peer_count = peer_addrs.len(), "Received peer addresses");

        let downloaded = self.download_from(peer_addrs, peer_id, event_sender)?;
        info!(
            file_bytes = hex::encode(&downloaded.content[..128]),
            file_size = downloaded.content.len(),
            download_duration = format!("{:.2?}", downloaded.download_duration),
            "Downloaded file"
        );

        Ok(())
    }

    pub fn download_from(
        self,
        peer_addrs: Vec<SocketAddr>,
        peer_id: PeerId,
        event_sender: &Sender<AppEvent>,
    ) -> Result<DownloadedFile> {
        let info = self.info;

        info!("Probing peers");
        let result = if let Some(mut channel) =
            probe_peers_sequential(&peer_addrs, |addr, cur_idx| {
                event_sender.send(AppEvent::Probing {
                    address: *addr,
                    current_index: cur_idx,
                    total_count: peer_addrs.len(),
                })?;
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
                .with_progress_callback(|current, total| {
                    let _ = event_sender
                        .send(AppEvent::Downloading(current, total))
                        .inspect_err(|e| error!(%e, "Failed to send downloading event"));
                })
                .download()
            })?;
            Ok(DownloadedFile {
                content: file_content,
                download_duration,
            })
        } else {
            Err("No peer responded".into())
        };

        event_sender.send(AppEvent::Completed)?;
        result
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
