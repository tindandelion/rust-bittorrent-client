use std::fs;
use std::net::SocketAddr;
use std::sync::mpsc;

use bt_client::Torrent;
use bt_client::result::Result;
use bt_client::types::PeerId;

const DATA_FILE_PATH: &str = "test-env/war-and-peace/war-and-peace.txt";
const TORRENT_FILE_PATH: &str = "test-env/war-and-peace/war-and-peace.torrent";
const LOCAL_IP_ADDRESS: &str = "127.0.0.1:51413";

#[test]
fn test_download_file() -> Result<()> {
    let peer_address = LOCAL_IP_ADDRESS.parse()?;
    let torrent = Torrent::read_file(TORRENT_FILE_PATH)?;
    let peer_id = PeerId::default();

    let (tx, _rx) = mpsc::channel();
    let downloaded = torrent.download_from(peer_id, vec![peer_address], &tx)?;
    assert_eq!(read_test_file()?, downloaded.content);

    Ok(())
}

fn read_test_file() -> Result<Vec<u8>> {
    let content = fs::read(DATA_FILE_PATH)?;
    Ok(content)
}
