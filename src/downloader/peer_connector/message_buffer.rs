use std::io;

use tracing::trace;

use crate::downloader::peer_comm::PeerMessage;

pub struct MessageBuffer {
    buffer: Vec<u8>,
    msg_length: Option<usize>,
}

impl MessageBuffer {
    const BUFFER_SIZE: usize = 10;

    pub fn new() -> Self {
        Self {
            buffer: vec![],
            msg_length: None,
        }
    }

    pub fn read(&mut self, src: &mut impl io::Read) -> io::Result<PeerMessage> {
        let buffered_message = self.process_buffered_data();
        if let Some(message) = buffered_message {
            trace!(?message, "returning previously buffered message");
            return Ok(message);
        }

        let mut buffer = [0; Self::BUFFER_SIZE];
        loop {
            let res = src.read(&mut buffer);
            match res {
                Ok(n) => {
                    trace!(num_bytes = n, "received bytes");
                    self.buffer.extend_from_slice(&buffer[..n]);
                    let buffered_message = self.process_buffered_data();
                    if let Some(message) = buffered_message {
                        trace!(?message, "returning received message");
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
