mod file_info;
mod piece_composer;
mod request_emitter;

use std::{io, time::Instant};

use crate::types::Sha1;
use file_info::FileInfo;
use piece_composer::{Piece, PieceComposer};
use request_emitter::RequestEmitter;
use tracing::debug;

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
    piece_composer: PieceComposer,
    request_emitter: RequestEmitter,
    tracker: DownloadTracker<'a>,
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
            piece_composer: PieceComposer::new(file_info),
            request_emitter: RequestEmitter::new(Self::BLOCK_LENGTH, file_info),
            tracker: DownloadTracker::new(file_info),
        }
    }

    #[cfg(test)]
    fn with_block_length(mut self, block_length: u32) -> Self {
        self.request_emitter.set_block_length(block_length);
        self
    }

    fn with_progress_callback(mut self, callback: impl FnMut(usize, usize) + 'a) -> Self {
        self.tracker.progress_callback = Box::new(callback);
        self
    }

    pub fn download(mut self) -> io::Result<Vec<u8>> {
        self.request_emitter
            .request_first_blocks(Self::REQUEST_QUEUE_LENGTH, self.channel)?;

        while self.tracker.has_more_pieces_to_download() {
            self.tracker.waiting_for_block();
            let block = self.channel.receive()?;
            self.request_emitter.request_next_block(self.channel)?;

            if let Some(piece) = self.piece_composer.append_block(&block)? {
                self.verify_piece_hash(&piece)?;
                self.tracker.append_piece(&piece);
            }
        }

        Ok(self.tracker.buffer)
    }

    fn verify_piece_hash(&self, piece: &Piece) -> io::Result<()> {
        let piece_hash = &self.piece_hashes[piece.index as usize];
        if !piece_hash.verify(&piece.data) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Downloaded piece does not match expected hash",
            ));
        }
        Ok(())
    }
}

struct DownloadTracker<'a> {
    progress_callback: Box<dyn FnMut(usize, usize) + 'a>,
    start_timestamp: Option<Instant>,
    downloaded_pieces: u32,
    downloaded_bytes: usize,
    file_info: FileInfo,
    buffer: Vec<u8>,
}

impl<'a> DownloadTracker<'a> {
    fn new(file_info: FileInfo) -> Self {
        Self {
            file_info,
            start_timestamp: None,
            downloaded_pieces: 0,
            downloaded_bytes: 0,
            progress_callback: Box::new(|_, _| {}),
            buffer: vec![0; file_info.file_length],
        }
    }

    fn waiting_for_block(&mut self) {
        if self.start_timestamp.is_none() {
            self.start_timestamp = Some(Instant::now());
        }
    }

    fn has_more_pieces_to_download(&self) -> bool {
        self.downloaded_pieces < self.file_info.piece_count()
    }

    fn append_piece(&mut self, piece: &Piece) {
        let (piece_start, piece_end) = self.file_info.piece_bounds(piece.index);
        self.buffer[piece_start..piece_end].copy_from_slice(&piece.data);
        self.piece_downloaded(piece);
    }

    fn piece_downloaded(&mut self, piece: &Piece) {
        self.downloaded_pieces += 1;
        self.downloaded_bytes += piece.data.len();
        let duration = self.start_timestamp.take().unwrap().elapsed();

        (self.progress_callback)(self.downloaded_bytes, self.file_info.file_length);
        debug!(
            piece_index = piece.index,
            duration_ms = duration.as_millis(),
            "Downloaded piece",
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
        let downloaded_data =
            FileDownloader::new(&mut channel, piece_hashes, piece_length, file_data.len())
                .with_block_length(3)
                .download()
                .unwrap();
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
        let downloaded_data =
            FileDownloader::new(&mut channel, piece_hashes, piece_length, file_data.len())
                .with_block_length(3)
                .download()
                .unwrap();
        assert_eq!(file_data, downloaded_data);
    }

    #[test]
    fn test_report_download_progress_via_provided_callback() {
        let file_data = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25,
        ];
        let file_length = file_data.len();
        let piece_length = 10_u32;
        let pieces = file_data
            .chunks(piece_length as usize)
            .map(|c| c.to_vec())
            .collect::<Vec<_>>();
        let piece_hashes = pieces
            .iter()
            .map(|p| Sha1::calculate(p))
            .collect::<Vec<_>>();
        let mut reported_progress: Vec<(usize, usize)> = vec![];

        let mut channel = DownloadChannelFromVector::new(pieces.clone());
        FileDownloader::new(&mut channel, piece_hashes, piece_length, file_data.len())
            .with_progress_callback(|downloaded, total| reported_progress.push((downloaded, total)))
            .with_block_length(3)
            .download()
            .unwrap();

        assert_eq!(
            reported_progress,
            vec![
                (10, file_length),
                (20, file_length),
                (file_length, file_length)
            ]
        );
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
        FileDownloader::new(&mut channel, piece_hashes, 10, 20)
            .with_block_length(3)
            .download()
            .unwrap();
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

        let error = FileDownloader::new(&mut channel, vec![Sha1::new([0; 20])], 3, 3)
            .with_block_length(3)
            .download()
            .unwrap_err();
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
