use crate::types::Sha1;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::fs;

#[derive(Deserialize, Serialize)]
pub struct Info {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    pub length: usize,
    pub pieces: ByteBuf,
}

#[derive(Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

pub fn read_torrent_file() -> Result<Torrent, Box<dyn std::error::Error>> {
    let contents = fs::read(TORRENT_FILE)?;
    let decoded = serde_bencode::from_bytes(&contents)?;
    Ok(decoded)
}

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

#[cfg(test)]
mod tests {

    use super::*;

    const INFO_HASH: &str = "6f4370df4304609a8793ce2b59178dcc8febf5e2";

    #[test]
    fn deserialize_torrent_file() {
        let torrent: Torrent = read_torrent_file().unwrap();
        assert_eq!(
            torrent.announce,
            "http://bttracker.debian.org:6969/announce"
        );

        let info = torrent.info;
        assert_eq!(info.piece_length, 262144);
        assert_eq!(info.length, 702545920);
        assert_eq!(
            info.piece_hashes().len(),
            (info.length / info.piece_length as usize)
        );
    }

    #[test]
    fn calculate_info_hash() {
        let torrent: Torrent = read_torrent_file().unwrap();
        assert_eq!(format!("{}", torrent.info.sha1()), INFO_HASH);
    }
}

impl Info {
    pub fn piece_hashes(&self) -> Vec<Sha1> {
        self.pieces
            .as_slice()
            .chunks_exact(20)
            .map(|chunk| Sha1::from_bytes(chunk))
            .collect()
    }

    pub fn sha1(&self) -> Sha1 {
        let serialized = serde_bencode::to_bytes(&self).unwrap();
        Sha1::calculate(&serialized)
    }
}
