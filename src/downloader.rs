use std::{
    error::Error,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use bincode::{
    Decode, Encode,
    error::{DecodeError, EncodeError},
};

use crate::types::{PeerId, Sha1};

pub struct FileDownloader {
    stream: TcpStream,
}

const PROTO_ID: &[u8; 19] = b"BitTorrent protocol";

#[derive(Debug, Encode, Decode, PartialEq, Eq)]
struct HandshakeMessage {
    pstrlen: u8,
    pstr: [u8; PROTO_ID.len()],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl HandshakeMessage {
    fn new(info_hash: Sha1, peer_id: PeerId) -> Self {
        Self {
            pstrlen: PROTO_ID.len() as u8,
            pstr: *PROTO_ID,
            reserved: [0; 8],
            info_hash: *info_hash.as_bytes(),
            peer_id: *peer_id.as_bytes(),
        }
    }

    fn receive(src: &mut impl Read) -> Result<Self, DecodeError> {
        bincode::decode_from_std_read(src, bincode::config::standard())
    }

    fn send(&self, stream: &mut impl Write) -> Result<(), EncodeError> {
        bincode::encode_into_std_write(self, stream, bincode::config::standard()).map(|_| ())
    }
}

impl FileDownloader {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn connect(addr: &SocketAddr) -> Result<FileDownloader, Box<dyn Error>> {
        let stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        Ok(FileDownloader { stream })
    }

    pub fn handshake(
        &mut self,
        info_hash: Sha1,
        peer_id: PeerId,
    ) -> Result<String, Box<dyn Error>> {
        HandshakeMessage::new(info_hash, peer_id).send(&mut self.stream)?;
        let response = HandshakeMessage::receive(&mut self.stream)?;
        Ok(String::from_utf8_lossy(&response.peer_id).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_and_receive_handshake_message() {
        let info_hash = Sha1::new([0x01; 20]);
        let peer_id = PeerId::new([0x02; 20]);
        let message_to_send = HandshakeMessage::new(info_hash, peer_id);

        let mut buffer = Vec::new();
        message_to_send.send(&mut buffer).unwrap();
        let received_message: HandshakeMessage =
            bincode::decode_from_std_read(&mut buffer.as_slice(), bincode::config::standard())
                .unwrap();

        assert_eq!(message_to_send, received_message);
    }
}
