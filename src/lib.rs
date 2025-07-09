use std::{
    error::Error,
    fs,
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use crate::bencoding::{
    decode_dict,
    types::{Dict, Sha1},
};
mod bencoding;
mod tracker;
pub use tracker::{AnnounceParams, Peer, get_peer_list_from_response, make_announce_request};

const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

pub fn read_torrent_file() -> Dict {
    let contents = fs::read(TORRENT_FILE).unwrap();
    decode_dict(&contents).unwrap()
}

pub fn connect_to_first_peer(peers: &[Peer]) -> Option<TcpStream> {
    peers
        .iter()
        .filter_map(|peer| peer.connect(Duration::from_secs(5)).ok())
        .next()
}

#[cfg(test)]
mod tests {
    use crate::bencoding::types::Sha1;

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
            peer_id: [0x00; 20],
        };

        let result = make_announce_request(TRACKER_URL, &request_params).unwrap();
        assert_eq!("d14:failure reason17:torrent not founde", result);
    }
}

const PROTO_ID: &str = "BitTorrent protocol";
const HANDSHAKE_BUFFER_LEN: usize = 49 + PROTO_ID.len();
const INFO_HASH_OFFSET: usize = 1 + PROTO_ID.len() + 8;
const PEER_ID_OFFSET: usize = INFO_HASH_OFFSET + 20;

fn make_handshake_buffer(info_hash: &Sha1, peer_id: &[u8]) -> [u8; HANDSHAKE_BUFFER_LEN] {
    let mut buffer = [0x00; HANDSHAKE_BUFFER_LEN];
    buffer[0] = PROTO_ID.len() as u8;
    buffer[1..=PROTO_ID.len()].copy_from_slice(PROTO_ID.as_bytes());
    buffer[INFO_HASH_OFFSET..PEER_ID_OFFSET].copy_from_slice(info_hash.as_bytes());
    buffer[PEER_ID_OFFSET..].copy_from_slice(peer_id);
    buffer
}

pub fn make_handshake(
    stream: &mut TcpStream,
    info_hash: &Sha1,
    peer_id: &[u8],
) -> Result<String, Box<dyn Error>> {
    let buffer = make_handshake_buffer(info_hash, peer_id);
    stream.write_all(&buffer)?;

    let mut response_buffer = [0x00; HANDSHAKE_BUFFER_LEN];
    stream.read_exact(&mut response_buffer)?;

    Ok(String::from_utf8_lossy(&response_buffer).to_string())
}

#[cfg(test)]
mod handshake_tests {
    use super::*;

    #[test]
    fn test_make_handshake_buffer() {
        let info_hash = Sha1::new([0x01; 20]);
        let peer_id = [0x02; 20];

        let constructed_buffer = make_handshake_buffer(&info_hash, &peer_id);
        assert_eq!(PROTO_ID.len(), constructed_buffer[0] as usize);
        assert_eq!(
            PROTO_ID,
            String::from_utf8_lossy(&constructed_buffer[1..=PROTO_ID.len()])
        );
        assert_eq!(
            info_hash.as_bytes(),
            &constructed_buffer[INFO_HASH_OFFSET..PEER_ID_OFFSET]
        );
        assert_eq!(peer_id, &constructed_buffer[PEER_ID_OFFSET..]);
    }
}
