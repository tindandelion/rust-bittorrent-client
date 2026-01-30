use crate::types::Sha1;
use serde::{Deserialize, Serialize};
use std::fs;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
#[serde(try_from = "InfoInternal")]
pub struct Info {
    pub sha1: Sha1,
    pub name: String,
    pub piece_length: u32,
    pub length: usize,
    pub pieces: Vec<Sha1>,
}

#[derive(Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

impl Torrent {
    const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

    pub fn read_default_file() -> Result<Torrent, Error> {
        let contents = fs::read(Self::TORRENT_FILE)?;
        let decoded = serde_bencode::from_bytes(&contents)?;
        Ok(decoded)
    }
}

#[derive(Deserialize, Serialize)]
struct InfoInternal {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    pub length: usize,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
}

impl TryFrom<InfoInternal> for Info {
    type Error = Error;

    fn try_from(info_internal: InfoInternal) -> Result<Info, Self::Error> {
        let sha1 = Sha1::calculate(&serde_bencode::to_bytes(&info_internal)?);
        let pieces = info_internal
            .pieces
            .chunks_exact(20)
            .map(Sha1::from_bytes)
            .collect::<Vec<_>>();

        Ok(Self {
            name: info_internal.name,
            piece_length: info_internal.piece_length,
            length: info_internal.length,
            pieces,
            sha1,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const INFO_HASH: &str = "6f4370df4304609a8793ce2b59178dcc8febf5e2";

    #[test]
    fn deserialize_torrent_file() {
        let torrent = Torrent::read_default_file().unwrap();
        assert_eq!(
            torrent.announce,
            "http://bttracker.debian.org:6969/announce"
        );

        let info = torrent.info;
        assert_eq!(info.piece_length, 262144);
        assert_eq!(info.length, 702545920);
        assert_eq!(
            info.pieces.len(),
            (info.length / info.piece_length as usize)
        );
    }

    #[test]
    fn calculate_info_hash() {
        let torrent = Torrent::read_default_file().unwrap();
        assert_eq!(format!("{}", torrent.info.sha1), INFO_HASH);
    }
}
