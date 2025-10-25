use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    #[serde(rename = "piece length")]
    piece_length: u32,
    length: u64,
}

#[derive(Deserialize)]
struct Torrent {
    announce: String,
    info: Info,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

    #[test]
    fn test_bencoding_serde() {
        let contents = fs::read(TORRENT_FILE).unwrap();
        let torrent: Torrent = serde_bencode::from_bytes(&contents).unwrap();
        assert_eq!(
            torrent.announce,
            "http://bttracker.debian.org:6969/announce"
        );
    }
}
