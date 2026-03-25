use std::io;

use crate::downloader::peer_comm::PeerMessage;

pub struct MessageBuffer {
    buffer: Vec<u8>,
    msg_length: Option<usize>,
}

impl MessageBuffer {
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

    pub fn new() -> Self {
        Self {
            buffer: vec![],
            msg_length: None,
        }
    }

    pub fn read(&mut self, src: &mut impl io::Read) -> io::Result<PeerMessage> {
        if let Some(message) = self.process_buffered_data() {
            return Ok(message);
        }

        let mut buffer = [0; Self::BUFFER_SIZE];
        loop {
            let res = src.read(&mut buffer);
            match res {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Received 0  bytes from source",
                    ));
                }
                Ok(n) => {
                    self.buffer.extend_from_slice(&buffer[..n]);
                    if let Some(message) = self.process_buffered_data() {
                        return Ok(message);
                    }
                }
                Err(err) => break Err(err),
            }
        }
    }

    fn process_buffered_data(&mut self) -> Option<PeerMessage> {
        if self.msg_length.is_none() && self.buffer.len() >= 4 {
            self.msg_length =
                Some(u32::from_be_bytes(self.buffer[0..4].try_into().unwrap()) as usize);
            self.buffer.drain(0..4);
        }

        if let Some(msg_len) = self.msg_length {
            if self.buffer.len() >= msg_len {
                self.msg_length = None;

                let unprocessed = self.buffer.split_off(msg_len);
                let message_buffer = std::mem::replace(&mut self.buffer, unprocessed);
                return Some(PeerMessage::from_bytes(&message_buffer));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_from_empty_buffer() {
        let mut buffer = MessageBuffer::new();
        let mut src: &[u8] = &[];
        let err = buffer.read(&mut src).expect_err("expected error");

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn reading_from_small_buffer() {
        let mut buffer = MessageBuffer::new();
        let mut src: &[u8] = &[0, 0];
        let err = buffer.read(&mut src).expect_err("expected error");

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
