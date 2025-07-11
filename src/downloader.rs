use std::{
    error::Error,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use crate::types::{PeerId, Sha1};

pub struct FileDownloader {
    stream: TcpStream,
}

const PROTOCOL_ID: &[u8; 19] = b"BitTorrent protocol";

#[derive(Debug, PartialEq, Default)]
#[repr(C, packed)]
struct HandshakeMessage {
    pstrlen: u8,
    pstr: [u8; PROTOCOL_ID.len()],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl HandshakeMessage {
    fn new(info_hash: Sha1, peer_id: PeerId) -> Self {
        Self {
            pstrlen: PROTOCOL_ID.len() as u8,
            pstr: *PROTOCOL_ID,
            reserved: [0; 8],
            info_hash: *info_hash.as_bytes(),
            peer_id: *peer_id.as_bytes(),
        }
    }

    fn receive(src: &mut impl Read) -> io::Result<Self> {
        let mut instance = Self::default();
        let buffer_ptr = &mut instance as *mut Self as *mut [u8; size_of::<Self>()];
        unsafe { src.read_exact(&mut *buffer_ptr)? };
        Ok(instance)
    }

    fn send(&self, dst: &mut impl Write) -> io::Result<()> {
        let buffer_ptr = self as *const Self as *const [u8; size_of::<Self>()];
        unsafe { dst.write_all(&*buffer_ptr) }
    }
}

impl FileDownloader {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const READ_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn connect(addr: &SocketAddr) -> Result<FileDownloader, Box<dyn Error>> {
        let stream = TcpStream::connect_timeout(addr, Self::CONNECT_TIMEOUT)?;
        stream.set_read_timeout(Some(Self::READ_TIMEOUT))?;
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
        let mut buffer = Vec::new();

        let message_to_send = HandshakeMessage::new(Sha1::new([0x01; 20]), PeerId::new([0x02; 20]));
        message_to_send.send(&mut buffer).unwrap();
        assert_eq!(
            vec![
                19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111,
                99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2
            ],
            buffer
        );

        let received_message = HandshakeMessage::receive(&mut buffer.as_slice()).unwrap();
        assert_eq!(message_to_send, received_message);
    }
}
