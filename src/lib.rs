use std::fs;
use url::{ParseError, Url};

use crate::bencoding::{Dict, decode_torrent_file};
mod bencoding;

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

pub struct TrackerRequestParams {
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}

pub fn read_torrent_file() -> Dict {
    let contents = fs::read(TORRENT_FILE).unwrap();
    decode_torrent_file(&contents).unwrap()
}

pub fn make_tracker_request_url(
    tracker_url: &str,
    request_params: &TrackerRequestParams,
) -> Result<Url, ParseError> {
    let mut url = Url::parse(tracker_url)?;

    let info_hash_str = unsafe { String::from_utf8_unchecked(request_params.info_hash.clone()) };
    let peer_id_str = unsafe { String::from_utf8_unchecked(request_params.peer_id.clone()) };

    url.query_pairs_mut()
        .append_pair("info_hash", &info_hash_str)
        .append_pair("peer_id", &peer_id_str)
        .append_pair("port", "6881")
        .append_pair("uploaded", "0")
        .append_pair("downloaded", "0")
        .append_pair("left", "0")
        .append_pair("compact", "0")
        .append_pair("no_peer_id", "0")
        .append_pair("event", "started");

    Ok(url)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_read_torrent_file() {
        let dict = read_torrent_file();
        assert_eq!(
            dict.get_string("announce"),
            Some("http://bttracker.debian.org:6969/announce")
        )
    }

    #[test]
    fn test_make_simple_tracker_request() {
        let tracker_url = "http://bttracker.debian.org:6969/announce";
        let request_params = TrackerRequestParams {
            info_hash: vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
                0xef, 0x12, 0x34, 0x56, 0x78, 0x9a,
            ],
            peer_id: vec![0x00; 20],
        };

        let url = make_tracker_request_url(tracker_url, &request_params).unwrap();

        let expected_params = [
            "info_hash=%124Vx%9A%BC%DE%F1%23Eg%89%AB%CD%EF%124Vx%9A",
            "peer_id=%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00",
            "port=6881",
            "uploaded=0",
            "downloaded=0",
            "left=0",
            "compact=0",
            "no_peer_id=0",
            "event=started",
        ]
        .join("&");
        let full_expected_url = tracker_url.to_owned() + "?" + &expected_params;
        assert_eq!(full_expected_url, url.to_string())
    }
}
