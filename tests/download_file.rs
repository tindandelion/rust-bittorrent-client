use std::net::SocketAddr;
use std::sync::mpsc;

use bt_client::Torrent;
use bt_client::result::Result;
use bt_client::types::PeerId;

#[test]
fn test_download_file() -> Result<()> {
    let peer_address: SocketAddr = "127.0.0.1:51413".parse()?;
    let torrent = Torrent::read_file("test-env/war-and-peace/war-and-peace.torrent")?;

    let peer_id = PeerId::default();
    let (tx, _rx) = mpsc::channel();
    torrent.download_from(peer_id, vec![peer_address], &tx)?;

    Ok(())
}
