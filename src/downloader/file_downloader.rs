use std::io;

use crate::types::Sha1;

#[derive(Debug, Clone)]
pub struct Block {
    pub piece_index: u32,
    pub offset: u32,
    pub data: Vec<u8>,
}

pub trait RequestChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()>;
}

pub trait DownloadChannel {
    fn receive(&mut self) -> io::Result<Block>;
}

pub struct FileDownloader<'a, T: RequestChannel + DownloadChannel> {
    channel: &'a mut T,
    piece_hashes: Vec<Sha1>,
    file_length: usize,
    piece_length: u32,
    block_length: u32,
}

impl<'a, T: RequestChannel + DownloadChannel> FileDownloader<'a, T> {
    const BLOCK_LENGTH: u32 = 1 << 14;

    pub fn new(
        channel: &'a mut T,
        piece_hashes: Vec<Sha1>,
        piece_length: u32,
        file_length: usize,
    ) -> Self {
        Self {
            channel,
            piece_hashes,
            file_length,
            piece_length,
            block_length: Self::BLOCK_LENGTH,
        }
    }

    pub fn download(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; self.file_length];

        for piece_index in 0..self.pieces_count() {
            println!("- Downloading piece {}", piece_index);
            let (piece_start, piece_end, piece_length) = self.calc_piece_bounds(piece_index);
            let download_start = std::time::Instant::now();
            let piece = self.download_piece(piece_index, piece_length)?;
            println!(
                "- Downloaded piece {}, time: {} ms",
                piece_index,
                download_start.elapsed().as_millis()
            );
            buffer[piece_start..piece_end].copy_from_slice(&piece);
        }

        Ok(buffer)
    }

    fn download_piece(&mut self, piece_index: u32, piece_length: u32) -> io::Result<Vec<u8>> {
        let buffer = self.download_piece_by_block(piece_index, piece_length)?;
        self.verify_piece_hash(piece_index, &buffer)?;

        let piece_hash = &self.piece_hashes[piece_index as usize];
        if !piece_hash.verify(&buffer) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Downloaded piece does not match expected hash",
            ));
        }

        Ok(buffer)
    }

    fn receive_block(&mut self) -> io::Result<Block> {
        let receive_start = std::time::Instant::now();
        print!("-- Receiving block: ");
        let block = self.channel.receive()?;
        println!("{} ms", receive_start.elapsed().as_millis());
        Ok(block)
    }

    fn verify_received_block(&self, block: &Block, expected_piece_index: u32) -> io::Result<()> {
        if block.piece_index != expected_piece_index {
            return Err(unexpected_piece_index(
                expected_piece_index,
                block.piece_index,
            ));
        }
        Ok(())
    }

    fn pieces_count(&self) -> u32 {
        self.piece_hashes.len() as u32
    }

    fn calc_piece_bounds(&self, piece_index: u32) -> (usize, usize, u32) {
        let piece_start = piece_index as usize * self.piece_length as usize;
        let mut piece_end = (piece_index as usize + 1) * self.piece_length as usize;
        if piece_end > self.file_length {
            piece_end = self.file_length;
        };
        (piece_start, piece_end, (piece_end - piece_start) as u32)
    }

    fn download_piece_by_block(
        &mut self,
        piece_index: u32,
        piece_length: u32,
    ) -> io::Result<Vec<u8>> {
        let mut emitter = RequestEmitter::new(self.block_length, piece_index, piece_length);
        let mut composer = PieceComposer::new(piece_length);

        emitter.request_first_blocks(self.channel)?;

        loop {
            let block = self.receive_block()?;
            self.verify_received_block(&block, piece_index)?;
            if let Some(piece) = composer.append_block(&block)? {
                return Ok(piece);
            }
            emitter.request_next_block(self.channel)?;
        }
    }

    fn verify_piece_hash(&self, piece_index: u32, piece: &[u8]) -> io::Result<()> {
        let piece_hash = &self.piece_hashes[piece_index as usize];
        if !piece_hash.verify(piece) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Downloaded piece does not match expected hash",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    fn with_block_length(mut self, block_length: u32) -> Self {
        self.block_length = block_length;
        self
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

fn unexpected_block_offset(expected: u32, actual: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unexpected block offset in response: expected {}, got {}",
            expected, actual
        ),
    )
}

struct RequestEmitter {
    queue_length: u8,
    block_length: u32,
    piece_index: u32,
    piece_length: u32,
    next_block_index: u32,
}

impl RequestEmitter {
    fn new(block_length: u32, piece_index: u32, piece_length: u32) -> Self {
        Self {
            queue_length: 10,
            block_length,
            piece_index,
            piece_length,
            next_block_index: 0,
        }
    }

    fn request_next_block(&mut self, channel: &mut impl RequestChannel) -> io::Result<()> {
        let block_count = self.piece_length.div_ceil(self.block_length);
        if self.next_block_index >= block_count {
            return Ok(());
        }

        let block_offset = self.next_block_index * self.block_length;
        let block_length = self.block_length.min(self.piece_length - block_offset);

        let request_start = std::time::Instant::now();
        print!("-- Requesting block: ");
        channel.request(self.piece_index, block_offset, block_length)?;
        println!("{} ms", request_start.elapsed().as_millis());

        self.next_block_index += 1;
        Ok(())
    }

    fn request_first_blocks(&mut self, channel: &mut impl RequestChannel) -> io::Result<()> {
        for _ in 0..self.queue_length {
            self.request_next_block(channel)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn with_queue_length(mut self, queue_length: u8) -> Self {
        self.queue_length = queue_length;
        self
    }
}

struct PieceComposer {
    buffer: Vec<u8>,
    piece_length: u32,
}

impl PieceComposer {
    fn new(piece_length: u32) -> Self {
        Self {
            piece_length,
            buffer: Vec::with_capacity(piece_length as usize),
        }
    }

    fn append_block(&mut self, block: &Block) -> io::Result<Option<Vec<u8>>> {
        self.verify_block_offset(block.offset)?;
        self.buffer.extend(&block.data);

        if self.buffer.len() >= self.piece_length as usize {
            Ok(Some(self.buffer.clone()))
        } else {
            Ok(None)
        }
    }

    fn verify_block_offset(&self, offset: u32) -> io::Result<()> {
        let expected_offset = self.buffer.len() as u32;
        if expected_offset != offset {
            return Err(unexpected_block_offset(expected_offset, offset));
        }
        Ok(())
    }
}

#[cfg(test)]
mod piece_composer_tests {
    use super::*;

    #[test]
    fn append_block_appends_block_data_to_buffer() {
        let mut composer = PieceComposer::new(10);
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
        assert_eq!(buffer, Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    }

    #[test]
    fn append_first_block_with_wrong_offset() {
        let mut composer = PieceComposer::new(10);
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
        let mut composer = PieceComposer::new(10);
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
}

#[cfg(test)]
mod request_emitter_tests {
    use super::*;

    #[test]
    fn request_next_block() {
        let block_length = 10;
        let piece_length = 100;
        let mut emitter = RequestEmitter::new(block_length, 0, piece_length);
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 10)]);
    }

    #[test]
    fn request_next_block_until_end_of_piece() {
        let block_length = 10;
        let piece_length = 15;
        let mut emitter = RequestEmitter::new(block_length, 0, piece_length);
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 5)]);
    }

    #[test]
    fn stop_requesting_blocks_past_end_of_piece() {
        let block_length = 10;
        let piece_length = 15;
        let mut emitter = RequestEmitter::new(block_length, 0, piece_length);
        let mut channel = RequestRecorder::new();

        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();
        emitter.request_next_block(&mut channel).unwrap();

        assert_eq!(channel.requests, vec![(0, 0, 10), (0, 10, 5)]);
    }

    #[test]
    fn request_first_blocks() {
        let block_length = 10;
        let piece_length = 100;
        let queue_length = 3;
        let mut emitter =
            RequestEmitter::new(block_length, 0, piece_length).with_queue_length(queue_length);
        let mut channel = RequestRecorder::new();

        emitter.request_first_blocks(&mut channel).unwrap();
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::types::Sha1;

    use super::*;

    #[test]
    fn test_download_all_pieces_when_all_pieces_have_same_length() {
        let file_data = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let piece_length: u32 = 10;
        let pieces = file_data
            .chunks(piece_length as usize)
            .map(|c| c.to_vec())
            .collect::<Vec<_>>();
        let piece_hashes = pieces
            .iter()
            .map(|p| Sha1::calculate(p))
            .collect::<Vec<_>>();

        let mut channel = DownloadChannelFromVector::new(pieces.clone());
        let mut piece_downloader =
            FileDownloader::new(&mut channel, piece_hashes, piece_length, file_data.len())
                .with_block_length(3);

        let downloaded_data = piece_downloader.download().unwrap();
        assert_eq!(file_data, downloaded_data);
    }

    #[test]
    fn test_download_all_pieces_when_last_piece_is_shorter() {
        let file_data = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25,
        ];
        let piece_length = 10_u32;
        let pieces = file_data
            .chunks(piece_length as usize)
            .map(|c| c.to_vec())
            .collect::<Vec<_>>();
        let piece_hashes = pieces
            .iter()
            .map(|p| Sha1::calculate(p))
            .collect::<Vec<_>>();

        let mut channel = DownloadChannelFromVector::new(pieces.clone());
        let mut piece_downloader =
            FileDownloader::new(&mut channel, piece_hashes, piece_length, file_data.len())
                .with_block_length(3);

        let downloaded_data = piece_downloader.download().unwrap();
        assert_eq!(file_data, downloaded_data);
    }

    #[test]
    #[should_panic]
    fn test_downloaded_piece_does_not_match_expected_hash() {
        let pieces = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        ];
        let piece_hashes = pieces.iter().map(|_p| zero_sha1()).collect::<Vec<_>>();

        let mut channel = DownloadChannelFromVector::new(pieces.clone());
        let mut piece_downloader =
            FileDownloader::new(&mut channel, piece_hashes, 10, 20).with_block_length(3);

        piece_downloader.download().unwrap();
    }

    #[test]
    fn test_unexpected_offset_in_response() {
        let mut channel = ErrorDownloadChannel {
            block_to_send: Block {
                piece_index: 0,
                offset: 1,
                data: vec![0xff; 3],
            },
        };

        let mut piece_downloader =
            FileDownloader::new(&mut channel, vec![Sha1::new([0; 20])], 3, 3).with_block_length(3);

        let error = piece_downloader.download().unwrap_err();
        assert_eq!(unexpected_block_offset(0, 1).to_string(), error.to_string());
    }

    #[test]
    fn test_unexpected_piece_index_in_response() {
        let mut channel = ErrorDownloadChannel {
            block_to_send: Block {
                piece_index: 1,
                offset: 0,
                data: vec![0xff; 3],
            },
        };

        let mut piece_downloader =
            FileDownloader::new(&mut channel, vec![zero_sha1()], 3, 3).with_block_length(3);

        let error = piece_downloader.download().unwrap_err();
        assert_eq!(unexpected_piece_index(0, 1).to_string(), error.to_string());
    }

    struct DownloadChannelFromVector {
        pieces: Vec<Vec<u8>>,
        requests: VecDeque<(u32, u32, u32)>,
    }

    impl DownloadChannelFromVector {
        fn new(pieces: Vec<Vec<u8>>) -> Self {
            Self {
                pieces,
                requests: VecDeque::new(),
            }
        }
    }

    impl RequestChannel for DownloadChannelFromVector {
        fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
            self.requests.push_back((piece_index, offset, length));
            Ok(())
        }
    }

    impl DownloadChannel for DownloadChannelFromVector {
        fn receive(&mut self) -> io::Result<Block> {
            if let Some((piece_index, offset, length)) = self.requests.pop_front() {
                let piece = &self.pieces[piece_index as usize];
                let data = piece[offset as usize..(offset + length) as usize].to_vec();
                Ok(Block {
                    piece_index,
                    offset,
                    data,
                })
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "No block requested"))
            }
        }
    }

    struct ErrorDownloadChannel {
        block_to_send: Block,
    }

    impl RequestChannel for ErrorDownloadChannel {
        fn request(&mut self, _piece_index: u32, _offset: u32, _length: u32) -> io::Result<()> {
            Ok(())
        }
    }

    impl DownloadChannel for ErrorDownloadChannel {
        fn receive(&mut self) -> io::Result<Block> {
            Ok(self.block_to_send.clone())
        }
    }

    fn zero_sha1() -> Sha1 {
        Sha1::new([0; 20])
    }
}
