mod decode_dict;
mod decode_string;

use crate::decode_dict::Dict;
use std::fs;

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

pub fn bt_client() -> String {
    let dict = read_torrent_file();
    dict.get_string("announce").unwrap().to_string()
}

fn read_torrent_file() -> Dict {
    let contents = fs::read(TORRENT_FILE).unwrap();
    let (first_dict, _) = decode_dict::decode_dict(&contents).unwrap();
    first_dict
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
