use std::io;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PeerMessage {
    Bitfield(Vec<u8>),
    Interested,
    Unchoke,
    Request {
        piece_index: u32,
        offset: u32,
        length: u32,
    },
    Piece {
        piece_index: u32,
        offset: u32,
        block: Vec<u8>,
    },
    Unknown {
        id: u8,
        payload: Vec<u8>,
    },
}

impl PeerMessage {
    const MESSAGE_LENGTH_SIZE: usize = 4;
    const MAX_MESSAGE_LENGTH: usize = 128 * 1024; // 128KB

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let id = bytes[0];
        let payload = &bytes[1..];
        match id {
            1 => Self::Unchoke,
            2 => Self::Interested,
            5 => Self::Bitfield(payload.to_vec()),
            7 => {
                let piece_index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let offset = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let block = payload[8..].to_vec();
                Self::Piece {
                    piece_index,
                    offset,
                    block,
                }
            }
            _ => Self::Unknown {
                id,
                payload: payload.to_vec(),
            },
        }
    }

    pub fn receive(src: &mut impl io::Read) -> io::Result<Self> {
        let msg_len = Self::read_message_length(src)?;
        let payload = Self::read_message_payload(src, msg_len)?;
        Ok(Self::from_bytes(&payload))
    }

    pub fn send(&self, dst: &mut impl io::Write) -> io::Result<()> {
        match self {
            Self::Bitfield(bitfield) => {
                let mut msg = vec![];
                msg.extend_from_slice(&(bitfield.len() as u32 + 1).to_be_bytes());
                msg.push(5);
                msg.extend_from_slice(&bitfield);
                dst.write_all(&msg)
            }
            Self::Interested => {
                let msg = vec![0, 0, 0, 1, 2];
                dst.write_all(&msg)
            }
            Self::Unchoke => {
                let msg = vec![0, 0, 0, 1, 1];
                dst.write_all(&msg)
            }
            Self::Request {
                piece_index,
                offset,
                length,
            } => {
                let mut msg = vec![0, 0, 0, 13, 6];
                msg.extend_from_slice(&piece_index.to_be_bytes());
                msg.extend_from_slice(&offset.to_be_bytes());
                msg.extend_from_slice(&length.to_be_bytes());
                dst.write_all(&msg)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Does not support sending message of type: {:?}", self),
            )),
        }
    }

    pub fn read_message_length(src: &mut impl io::Read) -> io::Result<usize> {
        let mut buffer = [0_u8; Self::MESSAGE_LENGTH_SIZE];
        let mut msg_len = 0;
        while msg_len == 0 {
            src.read_exact(&mut buffer)?;
            msg_len = u32::from_be_bytes(buffer) as usize;
        }

        if msg_len > Self::MAX_MESSAGE_LENGTH {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Message length is too big: {}", msg_len),
            ))
        } else {
            Ok(msg_len)
        }
    }

    fn read_message_payload(src: &mut impl io::Read, msg_len: usize) -> io::Result<Vec<u8>> {
        let mut payload_buffer = vec![0_u8; msg_len];
        src.read_exact(&mut payload_buffer)?;
        Ok(payload_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receive_bitfield_message() {
        let buffer = vec![
            0, 0, 0, 5, // Message length,
            5, // Message id,
            1, 2, 3, 4, // Bitfield payload
        ];
        let message = PeerMessage::receive(&mut buffer.as_slice()).unwrap();
        assert_eq!(PeerMessage::Bitfield(vec![1, 2, 3, 4]), message);
    }

    #[test]
    fn send_bitfield_message() {
        let mut buffer = Vec::new();
        let bitfield = vec![1, 2, 3, 4];

        PeerMessage::Bitfield(bitfield).send(&mut buffer).unwrap();
        assert_eq!(
            buffer,
            vec![
                0, 0, 0, 5, // Message length,
                5, // Message id,
                1, 2, 3, 4, // Bitfield payload
            ]
        );
    }

    #[test]
    fn receive_interested_message() {
        let buffer = vec![
            0, 0, 0, 1, // Message length
            2, // Message id
        ];

        let message = PeerMessage::receive(&mut buffer.as_slice()).unwrap();
        assert_eq!(PeerMessage::Interested, message);
    }

    #[test]
    fn send_interested_message() {
        let mut buffer = Vec::new();

        PeerMessage::Interested.send(&mut buffer).unwrap();
        assert_eq!(
            buffer,
            vec![
                0, 0, 0, 1, // Message length
                2  // Message id
            ]
        );
    }

    #[test]
    fn send_request_message() {
        let mut buffer = Vec::new();

        PeerMessage::Request {
            piece_index: 1,
            offset: 10,
            length: 128,
        }
        .send(&mut buffer)
        .unwrap();
        assert_eq!(
            buffer,
            vec![
                0, 0, 0, 13, // Message length
                6,  // Message id
                0, 0, 0, 1, // Piece index
                0, 0, 0, 10, // Offset
                0, 0, 0, 128, // Length
            ]
        );
    }

    #[test]
    fn receive_piece_message() {
        let buffer = vec![
            0, 0, 0, 13, // Message length
            7,  // Message id
            0, 0, 0, 1, // Piece index
            0, 0, 0, 10, // Offset
            1, 2, 3, 4, // Block
        ];
        let message = PeerMessage::receive(&mut buffer.as_slice()).unwrap();
        assert_eq!(
            PeerMessage::Piece {
                piece_index: 1,
                offset: 10,
                block: vec![1, 2, 3, 4]
            },
            message
        );
    }

    #[test]
    fn skip_keep_alive_messages() {
        let buffer = vec![
            0, 0, 0, 0, // First keep-alive
            0, 0, 0, 0, // Second keep-alive
            0, 0, 0, 13, // Message length
            7,  // Message id
            0, 0, 0, 1, // Piece index
            0, 0, 0, 10, // Offset
            1, 2, 3, 4, // Block
        ];
        let message = PeerMessage::receive(&mut buffer.as_slice()).unwrap();
        assert_eq!(
            PeerMessage::Piece {
                piece_index: 1,
                offset: 10,
                block: vec![1, 2, 3, 4]
            },
            message
        );
    }

    #[test]
    fn limit_message_length() {
        // 0x100000
        let buffer = vec![
            0x00, 0x10, 0x00, 0x00, // Message length
            2,    // Message id
        ];
        let result = PeerMessage::receive(&mut buffer.as_slice()).expect_err("Expected an error");
        assert_eq!(result.kind(), io::ErrorKind::InvalidData);
    }
}
