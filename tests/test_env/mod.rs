use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

use bt_client::Torrent;
use bt_client::result::Result;

use testcontainers::{
    Container, GenericBuildableImage, GenericImage,
    core::IntoContainerPort,
    core::WaitFor,
    runners::{SyncBuilder, SyncRunner},
};

const TEST_ENV_DIR: &str = "./test-env";

pub struct TestEnv {
    container: Container<GenericImage>,
}

impl TestEnv {
    pub fn start() -> Result<Self> {
        let image = GenericBuildableImage::new("bt-client-transmission", "latest")
            .with_dockerfile(Self::dockerfile_path())
            .with_file(Self::test_data_dir(), "./war-and-peace")
            .build_image()?;

        let container = image
            .with_exposed_port(51413.tcp())
            .with_wait_for(WaitFor::message_on_stdout("[ls.io-init] done."))
            .start()?;

        Ok(Self { container })
    }

    pub fn get_peer_address(&self) -> Result<SocketAddr> {
        let port = self.container.get_host_port_ipv4(51413.tcp())?;
        Ok(("127.0.0.1", port).to_socket_addrs()?.next().unwrap())
    }

    pub fn read_torrent_file() -> Result<Torrent> {
        Torrent::read_file(
            Self::test_data_dir()
                .join("war-and-peace.torrent")
                .to_str()
                .unwrap(),
        )
    }

    pub fn read_data_file() -> Result<Vec<u8>> {
        fs::read(Self::test_data_dir().join("war-and-peace.txt")).map_err(|e| e.into())
    }

    fn dockerfile_path() -> PathBuf {
        PathBuf::from(TEST_ENV_DIR).join("Dockerfile")
    }

    fn test_data_dir() -> PathBuf {
        PathBuf::from(TEST_ENV_DIR).join("war-and-peace")
    }
}
