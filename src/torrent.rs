use crate::types::Sha1;
use serde::{Deserialize, Deserializer, Serialize};
use serde_bytes::ByteBuf;
use std::fs;

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

pub fn read_torrent_file() -> Result<Torrent, Box<dyn std::error::Error>> {
    let contents = fs::read(TORRENT_FILE)?;
    let decoded = serde_bencode::from_bytes(&contents)?;
    Ok(decoded)
}

impl<'de> Deserialize<'de> for Info {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Serialize)]
        struct InfoInternal {
            pub name: String,
            #[serde(rename = "piece length")]
            pub piece_length: u32,
            pub length: usize,
            pub pieces: ByteBuf,
        }

        let info_internal = InfoInternal::deserialize(deserializer)?;
        let sha1 = Sha1::calculate(&serde_bencode::to_bytes(&info_internal).unwrap());
        let pieces = info_internal
            .pieces
            .as_slice()
            .chunks_exact(20)
            .map(Sha1::from_bytes)
            .collect::<Vec<_>>();

        Ok(Info {
            name: info_internal.name,
            piece_length: info_internal.piece_length,
            length: info_internal.length,
            pieces,
            sha1,
        })
    }
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
            info.pieces.len(),
            (info.length / info.piece_length as usize)
        );
    }

    #[test]
    fn calculate_info_hash() {
        let torrent: Torrent = read_torrent_file().unwrap();
        assert_eq!(format!("{}", torrent.info.sha1), INFO_HASH);
    }
}
