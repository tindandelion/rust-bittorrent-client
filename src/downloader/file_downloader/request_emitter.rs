use std::io;

use super::{FileInfo, RequestChannel};

pub struct RequestEmitter {
    block_length: u32,
    piece_index: u32,
    next_block_index: u32,
    file_info: FileInfo,
}

impl RequestEmitter {
    pub fn new(block_length: u32, file_info: FileInfo) -> Self {
        Self {
            block_length,
            piece_index: 0,
            next_block_index: 0,
            file_info,
        }
    }

    pub fn request_next_block(&mut self, channel: &mut impl RequestChannel) -> io::Result<()> {
        if self.piece_index >= self.file_info.piece_count() {
            return Ok(());
        }

        let piece_length = self.file_info.piece_length(self.piece_index);
        let block_count = piece_length.div_ceil(self.block_length);
        let block_offset = self.next_block_index * self.block_length;
        let block_length = self.block_length.min(piece_length - block_offset);

        channel.request(self.piece_index, block_offset, block_length)?;

        self.next_block_index += 1;
        if self.next_block_index >= block_count {
            self.next_block_index = 0;
            self.piece_index += 1;
        }

        Ok(())
    }

    pub fn request_first_blocks(
        &mut self,
        n_requests: u16,
        channel: &mut impl RequestChannel,
    ) -> io::Result<()> {
        for _ in 0..n_requests {
            self.request_next_block(channel)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn set_block_length(&mut self, block_length: u32) {
        self.block_length = block_length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_next_block() {
        let block_length = 10;
        let mut emitter = RequestEmitter::new(
            block_length,
            FileInfo {
                file_length: 1000,
                piece_length: 100,
            },
        );
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 10)]);
    }

    #[test]
    fn request_next_block_until_end_of_piece() {
        let block_length = 10;
        let mut emitter = RequestEmitter::new(
            block_length,
            FileInfo {
                file_length: 1000,
                piece_length: 15,
            },
        );
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 5)]);
    }

    #[test]
    fn proceeds_to_next_piece_when_current_is_finished() {
        let block_length = 10;
        let mut emitter = RequestEmitter::new(
            block_length,
            FileInfo {
                file_length: 1000,
                piece_length: 15,
            },
        );
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();

        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 5), (1, 0, 10)]);
    }

    #[test]
    fn stops_requesting_blocks_past_end_of_file() {
        let block_length = 10;
        let mut emitter = RequestEmitter::new(
            block_length,
            FileInfo {
                file_length: 15,
                piece_length: 10,
            },
        );
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();

        assert_eq!(channel.requests, vec![(0, 0, 10), (1, 0, 5)]);
    }

    #[test]
    fn request_first_blocks() {
        let block_length = 10;
        let queue_length = 3;
        let mut emitter = RequestEmitter::new(
            block_length,
            FileInfo {
                file_length: 1000,
                piece_length: 100,
            },
        );
        let mut channel = RequestRecorder::new();

        emitter
            .request_first_blocks(queue_length, &mut channel)
            .unwrap();
        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 10), (0, 20, 10)]);
    }

    struct RequestRecorder {
        requests: Vec<(u32, u32, u32)>,
    }

    impl RequestRecorder {
        fn new() -> Self {
            Self {
                requests: Vec::new(),
            }
        }
    }

    impl RequestChannel for RequestRecorder {
        fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
            self.requests.push((piece_index, offset, length));
            Ok(())
        }
    }
}
