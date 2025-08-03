use std::io;

use super::peer_comm::{MessageChannel, PeerMessage};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedMessage {
        expected: &'static str,
        received: PeerMessage,
    },
    Other(String),
}

type Result<T> = std::result::Result<T, Error>;

pub fn request_complete_file(channel: &mut impl MessageChannel) -> Result<()> {
    match channel.receive()? {
        PeerMessage::Bitfield(_) => Ok(()),
        other => unexpected_message_error("bitfield", other),
    }?;

    channel.send(&PeerMessage::Interested)?;

    match channel.receive()? {
        PeerMessage::Unchoke => Ok(()),
        other => unexpected_message_error("unchoke", other),
    }?;

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

    #[test]
    fn request_complete_file_message_sequence() {
        let mut channel =
            MockChannel::new(vec![PeerMessage::Bitfield(vec![]), PeerMessage::Unchoke]);

        request_complete_file(&mut channel).unwrap();
        assert_eq!(
            vec![
                ("recv", PeerMessage::Bitfield(vec![])),
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

        let result = request_complete_file(&mut channel);
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
            PeerMessage::Bitfield(vec![]),
            unexpected_message.clone(),
        ]);

        let result = request_complete_file(&mut channel);
        assert_eq!(
            unexpected_message_error("unchoke", unexpected_message),
            result
        );
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
