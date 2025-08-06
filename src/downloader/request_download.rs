use std::io;

use super::peer_comm::{MessageChannel, PeerMessage};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedMessage {
        expected: &'static str,
        received: PeerMessage,
    },
    BitfieldSizeMismatch {
        expected: usize,
        received: usize,
    },
    IncompleteFile,
    Other(String),
}

type Result<T> = std::result::Result<T, Error>;

pub fn request_complete_file(channel: &mut impl MessageChannel, piece_count: usize) -> Result<()> {
    match channel.receive()? {
        PeerMessage::Bitfield(bitfield) => check_bitfield_size(bitfield.len(), piece_count)
            .and_then(|_| check_bitfield_completeness(&bitfield, piece_count)),
        other => unexpected_message_error("bitfield", other),
    }?;

    channel.send(&PeerMessage::Interested)?;

    match channel.receive()? {
        PeerMessage::Unchoke => Ok(()),
        other => unexpected_message_error("unchoke", other),
    }?;

    Ok(())
}

fn check_bitfield_size(bitfield_size: usize, piece_count: usize) -> std::result::Result<(), Error> {
    let expected_bitfield_size = piece_count.div_ceil(8);
    if bitfield_size != expected_bitfield_size {
        Err(Error::BitfieldSizeMismatch {
            expected: expected_bitfield_size,
            received: bitfield_size,
        })
    } else {
        Ok(())
    }
}

fn check_bitfield_completeness(
    bitfield: &[u8],
    piece_count: usize,
) -> std::result::Result<(), Error> {
    for byte in &bitfield[..bitfield.len() - 1] {
        if *byte != 255 {
            return Err(Error::IncompleteFile);
        }
    }

    let mut pieces_in_last_byte = piece_count % 8;
    if pieces_in_last_byte == 0 {
        pieces_in_last_byte = 8;
    }
    let last_byte_mask = (128u8 as i8 >> (pieces_in_last_byte - 1)) as u8;
    let last_byte = bitfield[bitfield.len() - 1];
    if last_byte & last_byte_mask != last_byte_mask {
        return Err(Error::IncompleteFile);
    }

    Ok(())
}

fn unexpected_message_error(expected: &'static str, received: PeerMessage) -> Result<()> {
    Err(Error::UnexpectedMessage { expected, received })
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Other(error.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    const PIECE_COUNT: usize = 8;

    #[test]
    fn request_complete_file_message_sequence() {
        let mut channel =
            MockChannel::new(vec![PeerMessage::Bitfield(vec![255]), PeerMessage::Unchoke]);

        request_complete_file(&mut channel, PIECE_COUNT).unwrap();
        assert_eq!(
            vec![
                ("recv", PeerMessage::Bitfield(vec![255])),
                ("send", PeerMessage::Interested),
                ("recv", PeerMessage::Unchoke),
            ],
            channel.message_sequence
        );
    }

    #[test]
    fn error_when_first_received_message_is_not_bitfield() {
        let unexpected_message = PeerMessage::Unchoke;
        let mut channel = MockChannel::new(vec![unexpected_message.clone()]);

        let result = request_complete_file(&mut channel, PIECE_COUNT);
        assert_eq!(
            unexpected_message_error("bitfield", unexpected_message),
            result
        );
    }

    #[test]
    fn error_when_second_received_message_is_not_unchoke() {
        let unexpected_message = PeerMessage::Unknown {
            id: 1,
            payload: vec![],
        };
        let mut channel = MockChannel::new(vec![
            PeerMessage::Bitfield(vec![255]),
            unexpected_message.clone(),
        ]);

        let result = request_complete_file(&mut channel, PIECE_COUNT);
        assert_eq!(
            unexpected_message_error("unchoke", unexpected_message),
            result
        );
    }

    mod bitfield_data_errors {
        use super::*;

        #[test]
        fn bitfield_data_too_short() {
            let piece_count = 16;
            let mut channel = MockChannel::new(vec![PeerMessage::Bitfield(vec![255])]);

            let result = request_complete_file(&mut channel, piece_count);
            assert_eq!(
                Err(Error::BitfieldSizeMismatch {
                    expected: 2,
                    received: 1,
                }),
                result
            );
        }

        #[test]
        fn bitfield_data_too_long() {
            let piece_count = 16;
            let mut channel = MockChannel::new(vec![PeerMessage::Bitfield(vec![255, 255, 255])]);

            let result = request_complete_file(&mut channel, piece_count);
            assert_eq!(
                Err(Error::BitfieldSizeMismatch {
                    expected: 2,
                    received: 3,
                }),
                result
            );
        }

        #[test]
        fn bitfield_data_missing_intermediate_pieces() {
            let piece_count = 16;
            let mut channel = MockChannel::new(vec![PeerMessage::Bitfield(vec![128, 255])]);

            let result = request_complete_file(&mut channel, piece_count);
            assert_eq!(Err(Error::IncompleteFile), result);
        }

        #[test]
        fn bitfield_data_missing_last_piece() {
            let piece_count = 15;
            let mut channel = MockChannel::new(vec![PeerMessage::Bitfield(vec![255, 0b11111100])]);

            let result = request_complete_file(&mut channel, piece_count);
            assert_eq!(Err(Error::IncompleteFile), result);
        }

        #[test]
        fn bitfield_ignore_redundant_bits_in_last_byte() {
            let piece_count = 10;
            let mut channel = MockChannel::new(vec![
                PeerMessage::Bitfield(vec![255, 0b11001000]),
                PeerMessage::Unchoke,
            ]);

            let result = request_complete_file(&mut channel, piece_count);
            assert_eq!(Ok(()), result);
        }
    }

    struct MockChannel {
        to_send: Vec<PeerMessage>,
        message_sequence: Vec<(&'static str, PeerMessage)>,
    }

    impl MockChannel {
        fn new(messages_to_send: Vec<PeerMessage>) -> Self {
            Self {
                to_send: messages_to_send.into_iter().rev().collect(),
                message_sequence: vec![],
            }
        }
    }

    impl MessageChannel for MockChannel {
        fn receive(&mut self) -> io::Result<PeerMessage> {
            let message = self
                .to_send
                .pop()
                .ok_or(io::Error::other("No more messages to send"))?;
            self.message_sequence.push(("recv", message.clone()));
            Ok(message)
        }

        fn send(&mut self, msg: &PeerMessage) -> io::Result<()> {
            self.message_sequence.push(("send", msg.clone()));
            Ok(())
        }
    }
}
