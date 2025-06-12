use std::fs;

use crate::bencoding::{Dict, decode_torrent_file};
mod bencoding;

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

pub fn read_torrent_file() -> Dict {
    let contents = fs::read(TORRENT_FILE).unwrap();
    decode_torrent_file(&contents).unwrap()
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
}
