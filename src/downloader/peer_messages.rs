use std::io;

#[derive(Debug, PartialEq)]
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
    pub fn receive(src: &mut impl io::Read) -> io::Result<Self> {
        let msg_len = Self::read_message_length(src)?;
        let (id, payload) = Self::read_message_payload(src, msg_len)?;
        match id {
            1 => Ok(Self::Unchoke),
            5 => Ok(Self::Bitfield(payload)),
            7 => {
                let piece_index = u32::from_be_bytes(payload[0..4].try_into().unwrap());
                let offset = u32::from_be_bytes(payload[4..8].try_into().unwrap());
                let block = payload[8..].to_vec();
                Ok(Self::Piece {
                    piece_index,
                    offset,
                    block,
                })
            }
            _ => Ok(Self::Unknown { id, payload }),
        }
    }

    pub fn send(self, dst: &mut impl io::Write) -> io::Result<()> {
        match self {
            Self::Interested => {
                let msg = vec![0, 0, 0, 1, 2];
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

    fn read_message_length(src: &mut impl io::Read) -> io::Result<usize> {
        let mut buffer = [0_u8; 4];
        src.read_exact(&mut buffer)?;
        Ok(u32::from_be_bytes(buffer) as usize)
    }

    fn read_message_payload(src: &mut impl io::Read, msg_len: usize) -> io::Result<(u8, Vec<u8>)> {
        let mut id_buffer = [0_u8; 1];
        let mut payload_buffer = vec![0_u8; msg_len - 1];
        src.read_exact(&mut id_buffer)?;
        src.read_exact(&mut payload_buffer)?;
        Ok((id_buffer[0], payload_buffer))
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
}
