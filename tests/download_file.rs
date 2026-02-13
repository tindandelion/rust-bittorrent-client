use std::sync::mpsc;

use bt_client::result::Result;
use bt_client::types::PeerId;

mod test_env;
use test_env::TestEnv;

#[test]
fn download_file_successfully() -> Result<()> {
    let env = TestEnv::start()?;

    let peer_address = env.get_peer_address()?;
    let torrent = TestEnv::read_torrent_file()?;
    let peer_id = PeerId::default();

    let (tx, _rx) = mpsc::channel();
    let downloaded = torrent.download_from(vec![peer_address], peer_id, &tx)?;
    assert_eq!(TestEnv::read_data_file()?, downloaded.content);

    Ok(())
}

#[test]
fn fail_to_connect_to_peer() -> Result<()> {
    let peer_address = "127.0.0.1:12345".parse()?;
    let torrent = TestEnv::read_torrent_file()?;
    let peer_id = PeerId::default();

    let (tx, _rx) = mpsc::channel();
    torrent
        .download_from(vec![peer_address], peer_id, &tx)
        .expect_err("Expected error");

    Ok(())
}
