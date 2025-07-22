use std::io;

use crate::types::Sha1;

#[derive(Debug, Clone)]
pub struct Block {
    pub piece_index: u32,
    pub offset: u32,
    pub data: Vec<u8>,
}

pub trait PieceDownloadChannel {
    fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()>;
    fn receive(&mut self) -> io::Result<Block>;
}

pub struct PieceDownloader<T: PieceDownloadChannel> {
    channel: T,
    piece_hashes: Vec<Sha1>,
    block_length: u32,
    piece_length: u32,
}

impl<T: PieceDownloadChannel> PieceDownloader<T> {
    const BLOCK_LENGTH: u32 = 1 << 14;

    pub fn new(channel: T, piece_hashes: Vec<Sha1>, piece_length: u32) -> Self {
        Self {
            channel,
            piece_hashes,
            block_length: Self::BLOCK_LENGTH,
            piece_length,
        }
    }

    pub fn download_piece(&mut self, piece_index: u32) -> io::Result<Vec<u8>> {
        let buffer = self.download_piece_by_block(piece_index)?;
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

    fn request_block(&mut self, piece_index: u32, block_index: u32) -> io::Result<(u32, u32)> {
        let block_offset = block_index * self.block_length;
        let block_length = self.block_length.min(self.piece_length - block_offset);

        self.channel
            .request(piece_index, block_offset, block_length)?;
        Ok((block_offset, block_length))
    }

    fn receive_block(
        &mut self,
        piece_index: u32,
        block_offset: u32,
        block_length: u32,
    ) -> io::Result<Vec<u8>> {
        let block = self.channel.receive()?;
        self.verify_received_block(&block, piece_index, block_offset, block_length)?;
        Ok(block.data)
    }

    fn verify_received_block(
        &self,
        block: &Block,
        expected_piece_index: u32,
        expected_offset: u32,
        expected_length: u32,
    ) -> io::Result<()> {
        if block.piece_index != expected_piece_index {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected piece index in response: expected {}, got {}",
                    expected_piece_index, block.piece_index
                ),
            ));
        }

        if block.offset != expected_offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected block offset in response: expected {}, got {}",
                    expected_offset, block.offset
                ),
            ));
        }
        if block.data.len() != expected_length as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected block data length in response: expected {}, got {}",
                    expected_length,
                    block.data.len()
                ),
            ));
        }

        Ok(())
    }

    fn download_piece_by_block(&mut self, piece_index: u32) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; self.piece_length as usize];

        let block_count = (self.piece_length + self.block_length - 1) / self.block_length;
        for block_index in 0..block_count {
            let (block_offset, block_length) = self.request_block(piece_index, block_index)?;
            let data = self.receive_block(piece_index, block_offset, block_length)?;
            buffer[block_offset as usize..(block_offset + block_length) as usize]
                .copy_from_slice(&data);
        }
        Ok(buffer)
    }

    fn verify_piece_hash(&self, piece_index: u32, piece: &[u8]) -> io::Result<()> {
        let piece_hash = &self.piece_hashes[piece_index as usize];
        if !piece_hash.verify(&piece) {
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

#[cfg(test)]
mod tests {
    use crate::types::Sha1;

    use super::*;

    #[test]
    fn test_download_piece() {
        let pieces = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        ];
        let piece_hashes = pieces
            .iter()
            .map(|p| Sha1::calculate(p))
            .collect::<Vec<_>>();

        let channel = DownloadChannelFromVector::new(pieces.clone());
        let mut piece_downloader =
            PieceDownloader::new(channel, piece_hashes, 10).with_block_length(3);

        let downloaded_piece = piece_downloader.download_piece(1).unwrap();
        assert_eq!(pieces[1], downloaded_piece);
    }

    #[test]
    #[should_panic]
    fn test_downloaded_piece_does_not_match_expected_hash() {
        let pieces = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        ];
        let piece_hashes = pieces.iter().map(|_p| zero_sha1()).collect::<Vec<_>>();

        let channel = DownloadChannelFromVector::new(pieces.clone());
        let mut piece_downloader =
            PieceDownloader::new(channel, piece_hashes, 10).with_block_length(3);

        piece_downloader.download_piece(0).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_unexpected_offset_in_response() {
        let channel = ErrorDownloadChannel {
            block_to_send: Block {
                piece_index: 0,
                offset: 1,
                data: vec![0xff; 3],
            },
        };

        let mut piece_downloader =
            PieceDownloader::new(channel, vec![Sha1::new([0; 20])], 3).with_block_length(3);

        piece_downloader.download_piece(0).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_unexpected_data_length_in_response() {
        let channel = ErrorDownloadChannel {
            block_to_send: Block {
                piece_index: 0,
                offset: 0,
                data: vec![0xff; 2],
            },
        };

        let mut piece_downloader =
            PieceDownloader::new(channel, vec![zero_sha1()], 3).with_block_length(3);

        piece_downloader.download_piece(0).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_unexpected_piece_index_in_response() {
        let channel = ErrorDownloadChannel {
            block_to_send: Block {
                piece_index: 1,
                offset: 0,
                data: vec![0xff; 3],
            },
        };

        let mut piece_downloader =
            PieceDownloader::new(channel, vec![zero_sha1()], 3).with_block_length(3);

        piece_downloader.download_piece(0).unwrap();
    }

    struct DownloadChannelFromVector {
        pieces: Vec<Vec<u8>>,
        requested_block: Option<(u32, u32, u32)>,
    }

    impl DownloadChannelFromVector {
        fn new(pieces: Vec<Vec<u8>>) -> Self {
            Self {
                pieces,
                requested_block: None,
            }
        }
    }

    impl PieceDownloadChannel for DownloadChannelFromVector {
        fn request(&mut self, piece_index: u32, offset: u32, length: u32) -> io::Result<()> {
            assert!(self.requested_block.is_none());
            self.requested_block = Some((piece_index, offset, length));
            Ok(())
        }

        fn receive(&mut self) -> io::Result<Block> {
            if let Some((piece_index, offset, length)) = self.requested_block {
                self.requested_block = None;

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

    impl PieceDownloadChannel for ErrorDownloadChannel {
        fn request(&mut self, _piece_index: u32, _offset: u32, _length: u32) -> io::Result<()> {
            Ok(())
        }

        fn receive(&mut self) -> io::Result<Block> {
            Ok(self.block_to_send.clone())
        }
    }

    fn zero_sha1() -> Sha1 {
        Sha1::new([0; 20])
    }
}
