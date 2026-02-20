mod downloader;
pub mod ratatui_ui;
pub mod result;
mod torrent;
mod tracker;
pub mod types;
mod util;

use tracing::{Level, debug, error, info, instrument};

use crate::{
    downloader::{PeerChannel, peer_connectors::SeqPeerConnector},
    ratatui_ui::AppEvent,
    torrent::Info,
    tracker::AnnounceRequest,
    types::PeerId,
};
use result::Result;
use std::{
    net::{SocketAddr, TcpStream},
    sync::mpsc::Sender,
    time::Duration,
};
pub use torrent::Torrent;

#[derive(Debug)]
pub struct DownloadedFile {
    pub content: Vec<u8>,
    pub download_duration: Duration,
}

impl Torrent {
    pub fn fetch_peer_addresses(&self, peer_id: PeerId) -> Result<Vec<SocketAddr>> {
        let announce_request = AnnounceRequest {
            tracker_url: self.announce.clone(),
            info_hash: self.info.sha1,
            peer_id,
        };
        announce_request.fetch_peer_addresses()
    }

    pub fn request_file_from_address(
        &self,
        addr: SocketAddr,
        peer_id: PeerId,
    ) -> Result<PeerChannel> {
        let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)?;
        request_complete_file(stream, &peer_id, &self.info)
    }

    fn request_file(
        &self,
        peer_addrs: Vec<SocketAddr>,
        peer_id: PeerId,
        event_sender: &Sender<AppEvent>,
    ) -> Option<PeerChannel> {
        let total_peers = peer_addrs.len();
        let connector = SeqPeerConnector::default().with_progress_callback(|addr, total_probed| {
            let _ = event_sender
                .send(AppEvent::Probing {
                    address: addr,
                    current_index: total_probed,
                    total_count: total_peers,
                })
                .inspect_err(|e| error!(%e, "Failed to send probing event"));
        });
        connector
            .connect(peer_addrs)
            .map(|stream| request_complete_file(stream, &peer_id, &self.info))
            .filter_map(Result::ok)
            .next()
    }

    pub fn download(self, event_sender: &Sender<AppEvent>) -> Result<()> {
        let peer_id = PeerId::default();
        let peer_addrs = self.fetch_peer_addresses(peer_id)?;
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
        let info = &self.info;

        info!("Probing peers");
        let result = if let Some(mut channel) = self.request_file(peer_addrs, peer_id, event_sender)
        {
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
                    info.pieces.clone(),
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

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[instrument(skip_all, level = Level::DEBUG)]
fn request_complete_file(stream: TcpStream, peer_id: &PeerId, info: &Info) -> Result<PeerChannel> {
    let mut channel = PeerChannel::handshake(stream, &info.sha1, peer_id)
        .inspect(|channel| debug!(remote_id = %channel.remote_id(), "Connected"))?;

    debug!("Connected, requesting file");
    downloader::request_complete_file(&mut channel, info.pieces.len())?;
    debug!("Ready to download");
    Ok(channel)
}
