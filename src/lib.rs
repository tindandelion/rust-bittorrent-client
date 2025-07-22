mod bencoding;
pub mod downloader;
mod tracker;
pub mod types;

use crate::{
    bencoding::{
        decode_dict,
        types::{ByteString, Dict},
    },
    types::Sha1,
};
use std::fs;
pub use tracker::{AnnounceParams, get_peer_list_from_response, make_announce_request};

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

pub fn read_torrent_file() -> Dict {
    let contents = fs::read(TORRENT_FILE).unwrap();
    decode_dict(&contents).unwrap()
}

pub fn get_piece_hashes(pieces: &ByteString) -> Vec<Sha1> {
    pieces
        .as_slice()
        .chunks_exact(20)
        .map(Sha1::from_bytes)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::types::{PeerId, Sha1};

    use super::*;

    const TRACKER_URL: &str = "http://bttracker.debian.org:6969/announce";

    #[test]
    fn test_read_torrent_file() {
        let dict = read_torrent_file();
        assert_eq!(
            TRACKER_URL,
            dict.get("announce")
                .and_then(|v| v.as_byte_string())
                .unwrap()
                .to_string()
        )
    }

    #[test]
    fn test_make_announce_request() {
        let request_params = AnnounceParams {
            info_hash: Sha1::new([0x00; 20]),
            peer_id: PeerId::default(),
        };

        let result = make_announce_request(TRACKER_URL, &request_params).unwrap();
        assert_eq!("d14:failure reason17:torrent not founde", result);
    }
}
