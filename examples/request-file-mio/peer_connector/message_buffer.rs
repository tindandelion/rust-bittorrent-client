use std::io;

use bt_client::downloader::peer_comm::PeerMessage;

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

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn read(&mut self, src: &mut impl io::Read) -> io::Result<PeerMessage> {
        if let Some(message) = self.process_buffered_data()? {
            return Ok(message);
        }

        let mut buffer = [0; Self::BUFFER_SIZE];
        loop {
            let res = src.read(&mut buffer);
            match res {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Received zero bytes from source",
                    ));
                }
                Ok(n) => {
                    self.buffer.extend_from_slice(&buffer[..n]);
                    if let Some(message) = self.process_buffered_data()? {
                        return Ok(message);
                    }
                }
                Err(err) => break Err(err),
            }
        }
    }

    fn process_buffered_data(&mut self) -> io::Result<Option<PeerMessage>> {
        if self.msg_length.is_none() && self.buffer.len() >= 4 {
            self.read_message_length()?;
        }

        if let Some(msg_len) = self.msg_length
            && self.buffer.len() >= msg_len
        {
            self.msg_length = None;

            let unprocessed = self.buffer.split_off(msg_len);
            let message_buffer = std::mem::replace(&mut self.buffer, unprocessed);
            return Ok(Some(PeerMessage::from_bytes(&message_buffer)));
        }

        Ok(None)
    }

    fn read_message_length(&mut self) -> io::Result<()> {
        let msg_length = u32::from_be_bytes(self.buffer[0..4].try_into().unwrap()) as usize;
        if msg_length > PeerMessage::MAX_MESSAGE_LENGTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message length is too big: {}", msg_length),
            ));
        }

        self.buffer.drain(0..4);
        self.msg_length = Some(msg_length);
        Ok(())
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

        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn reading_from_small_buffer() {
        let mut buffer = MessageBuffer::new();
        let mut src: &[u8] = &[0, 0];
        let err = buffer.read(&mut src).expect_err("expected error");

        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn reading_too_large_message_length() {
        let too_large_length = PeerMessage::MAX_MESSAGE_LENGTH + 1;
        let mut buffer = MessageBuffer::new();
        let src = (too_large_length as u32).to_be_bytes();
        let mut src = &src[..];
        println!("***  src length: {}", src.len());
        let err = buffer.read(&mut src).unwrap_err();

        assert_eq!(
            err.to_string(),
            format!("Message length is too big: {}", too_large_length)
        );
    }
}
