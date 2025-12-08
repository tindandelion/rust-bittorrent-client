use super::{Block, FileInfo};
use std::io;

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub index: u32,
    pub data: Vec<u8>,
}

impl Piece {
    fn new(index: u32, data: Vec<u8>) -> Self {
        Self { index, data }
    }
}

pub struct PieceComposer {
    piece_index: Option<u32>,
    buffer: Vec<u8>,
    file_info: FileInfo,
}

impl PieceComposer {
    pub fn new(file_info: FileInfo) -> Self {
        Self {
            buffer: Vec::with_capacity(file_info.piece_length as usize),
            file_info,
            piece_index: None,
        }
    }

    pub fn append_block(&mut self, block: &Block) -> io::Result<Option<Piece>> {
        if self.piece_index.is_none() {
            self.piece_index = Some(block.piece_index);
        }

        self.verify_piece_index(block.piece_index)?;
        self.verify_block_offset(block.offset)?;
        self.buffer.extend(&block.data);

        if self.buffer.len() >= self.current_piece_length() {
            let piece = Piece::new(self.piece_index.unwrap(), self.buffer.clone());
            self.buffer.clear();
            self.piece_index = None;
            Ok(Some(piece))
        } else {
            Ok(None)
        }
    }

    fn current_piece_length(&self) -> usize {
        self.file_info.piece_length(self.piece_index.unwrap()) as usize
    }

    fn verify_block_offset(&self, offset: u32) -> io::Result<()> {
        let expected_offset = self.buffer.len() as u32;
        if expected_offset != offset {
            return Err(unexpected_block_offset(expected_offset, offset));
        }
        Ok(())
    }

    fn verify_piece_index(&self, piece_index: u32) -> io::Result<()> {
        let expected = self.piece_index.unwrap();
        if expected != piece_index {
            return Err(unexpected_piece_index(expected, piece_index));
        }
        Ok(())
    }
}

fn unexpected_piece_index(expected: u32, actual: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unexpected piece index in response: expected {}, got {}",
            expected, actual
        ),
    )
}

pub fn unexpected_block_offset(expected: u32, actual: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unexpected block offset in response: expected {}, got {}",
            expected, actual
        ),
    )
}

#[cfg(test)]
mod piece_composer_tests {
    use super::*;

    #[test]
    fn compose_piece_from_blocks() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 100,
        });
        let first_block = Block {
            piece_index: 0,
            offset: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let second_block = Block {
            piece_index: 0,
            offset: 5,
            data: vec![6, 7, 8, 9, 10],
        };

        let buffer = composer.append_block(&first_block).unwrap();
        assert_eq!(buffer, None);

        let buffer = composer.append_block(&second_block).unwrap();
        assert_eq!(
            buffer,
            Some(Piece::new(0, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
        );
    }

    #[test]
    fn compose_last_piece_with_reduced_length_from_blocks() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 17,
        });
        let last_piece_index = 1;
        let first_block = Block {
            piece_index: last_piece_index,
            offset: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let second_block = Block {
            piece_index: last_piece_index,
            offset: 5,
            data: vec![6, 7],
        };

        let buffer = composer.append_block(&first_block).unwrap();
        assert_eq!(buffer, None);

        let buffer = composer.append_block(&second_block).unwrap();
        assert_eq!(
            buffer,
            Some(Piece::new(last_piece_index, vec![1, 2, 3, 4, 5, 6, 7]))
        );
    }

    #[test]
    fn starts_composing_new_piece_when_current_is_finished() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 17,
        });
        let first_block = Block {
            piece_index: 0,
            offset: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let second_block = Block {
            piece_index: 0,
            offset: 5,
            data: vec![6, 7, 8, 9, 10],
        };

        let third_block = Block {
            piece_index: 1,
            offset: 0,
            data: vec![11, 12, 13, 14, 15, 16, 17],
        };

        let buffer = composer.append_block(&first_block).unwrap();
        assert_eq!(buffer, None);

        let buffer = composer.append_block(&second_block).unwrap();
        assert_eq!(
            buffer,
            Some(Piece::new(0, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
        );

        let buffer = composer.append_block(&third_block).unwrap();
        assert_eq!(
            buffer,
            Some(Piece::new(1, vec![11, 12, 13, 14, 15, 16, 17]))
        );
    }

    #[test]
    fn append_first_block_with_wrong_offset() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 100,
        });
        let block = Block {
            piece_index: 0,
            offset: 1,
            data: vec![1, 2, 3, 4, 5],
        };
        let error = composer.append_block(&block).unwrap_err();
        assert_eq!(unexpected_block_offset(0, 1).to_string(), error.to_string());
    }

    #[test]
    fn append_next_block_with_wrong_offset() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 100,
        });
        let first_block = Block {
            piece_index: 0,
            offset: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let second_block = Block {
            piece_index: 0,
            offset: 3,
            data: vec![6, 7, 8, 9, 10],
        };

        composer.append_block(&first_block).unwrap();
        let error = composer.append_block(&second_block).unwrap_err();
        assert_eq!(unexpected_block_offset(5, 3).to_string(), error.to_string());
    }

    #[test]
    fn append_next_block_with_wrong_piece_index() {
        let mut composer = PieceComposer::new(FileInfo {
            piece_length: 10,
            file_length: 100,
        });
        let first_block = Block {
            piece_index: 0,
            offset: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let second_block = Block {
            piece_index: 1,
            offset: 5,
            data: vec![6, 7, 8, 9, 10],
        };

        composer.append_block(&first_block).unwrap();
        let error = composer.append_block(&second_block).unwrap_err();
        assert_eq!(unexpected_piece_index(0, 1).to_string(), error.to_string());
    }
}
