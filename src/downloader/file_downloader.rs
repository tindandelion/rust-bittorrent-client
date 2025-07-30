mod piece_composer;
mod request_emitter;

use std::{io, time::Instant};

use crate::types::Sha1;
use piece_composer::PieceComposer;
use request_emitter::RequestEmitter;

#[derive(Debug, Clone)]
pub struct Block {
    pub piece_index: u32,
    pub offset: u32,
    pub data: Vec<u8>,
}

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

pub trait RequestChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()>;
}

pub trait DownloadChannel {
    fn receive(&mut self) -> io::Result<Block>;
}

#[derive(Debug, Clone, Copy)]
pub struct FileInfo {
    file_length: usize,
    piece_length: u32,
}

impl FileInfo {
    fn piece_length(&self, piece_index: u32) -> u32 {
        let (piece_start, piece_end) = self.piece_bounds(piece_index);
        (piece_end - piece_start) as u32
    }

    fn piece_count(&self) -> u32 {
        self.file_length.div_ceil(self.piece_length as usize) as u32
    }

    fn piece_bounds(&self, piece_index: u32) -> (usize, usize) {
        let piece_start = piece_index as usize * self.piece_length as usize;
        let mut piece_end = (piece_index as usize + 1) * self.piece_length as usize;
        if piece_end > self.file_length {
            piece_end = self.file_length;
        };
        (piece_start, piece_end)
    }
}

pub struct FileDownloader<'a, T: RequestChannel + DownloadChannel> {
    channel: &'a mut T,
    piece_hashes: Vec<Sha1>,
    file_info: FileInfo,
    piece_composer: PieceComposer,
    request_emitter: RequestEmitter,
}

impl<'a, T: RequestChannel + DownloadChannel> FileDownloader<'a, T> {
    const REQUEST_QUEUE_LENGTH: u16 = 150;
    const BLOCK_LENGTH: u32 = 1 << 14;

    pub fn new(
        channel: &'a mut T,
        piece_hashes: Vec<Sha1>,
        piece_length: u32,
        file_length: usize,
    ) -> Self {
        let file_info = FileInfo {
            piece_length,
            file_length,
        };
        Self {
            channel,
            piece_hashes,
            file_info,
            piece_composer: PieceComposer::new(file_info),
            request_emitter: RequestEmitter::new(Self::BLOCK_LENGTH, file_info),
        }
    }

    pub fn download(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; self.file_info.file_length];
        let mut downloaded_pieces_count = 0;
        let mut download_report = DownloadReport::new();

        self.request_emitter
            .request_first_blocks(Self::REQUEST_QUEUE_LENGTH, self.channel)?;

        while downloaded_pieces_count < self.file_info.piece_count() {
            download_report.download_started();
            let block = self.channel.receive()?;
            self.request_emitter.request_next_block(self.channel)?;

            if let Some(piece) = self.piece_composer.append_block(&block)? {
                self.verify_piece_hash(piece.index, &piece)?;

                let (piece_start, piece_end) = self.file_info.piece_bounds(piece.index);
                buffer[piece_start..piece_end].copy_from_slice(&piece.data);

                download_report.download_finished(piece.index);
                downloaded_pieces_count += 1;
            }
        }

        Ok(buffer)
    }

    fn verify_piece_hash(&self, piece_index: u32, piece: &Piece) -> io::Result<()> {
        let piece_hash = &self.piece_hashes[piece_index as usize];
        if !piece_hash.verify(&piece.data) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Downloaded piece does not match expected hash",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    fn with_block_length(mut self, block_length: u32) -> Self {
        self.request_emitter.set_block_length(block_length);
        self
    }
}

struct DownloadReport {
    start_timestamp: Option<Instant>,
}

impl DownloadReport {
    fn new() -> Self {
        Self {
            start_timestamp: None,
        }
    }

    fn download_started(&mut self) {
        if self.start_timestamp.is_none() {
            self.start_timestamp = Some(Instant::now());
        }
    }

    fn download_finished(&mut self, piece_index: u32) {
        let duration = self.start_timestamp.take().unwrap().elapsed();
        println!(
            "- Downloaded piece {}: {} ms",
            piece_index,
            duration.as_millis()
        );
    }
}

#[cfg(test)]
mod tests {
    use piece_composer::unexpected_block_offset;
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
