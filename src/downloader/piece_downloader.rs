use std::io;

pub struct Block {
    pub offset: usize,
    pub data: Vec<u8>,
}

pub trait DownloadChannel {
    fn request(&mut self, offset: usize, length: usize) -> io::Result<()>;
    fn receive(&mut self) -> io::Result<Block>;
}

pub struct PieceDownloader<T: DownloadChannel> {
    channel: T,
    block_length: usize,
    buffer: Vec<u8>,
}

impl<T: DownloadChannel> PieceDownloader<T> {
    pub fn new(channel: T, piece_length: usize, block_length: usize) -> Self {
        Self {
            channel,
            block_length,
            buffer: vec![0; piece_length],
        }
    }

    fn download_piece(&mut self) -> io::Result<Vec<u8>> {
        let block_count = (self.buffer.len() + self.block_length - 1) / self.block_length;
        for block_index in 0..block_count {
            let (block_offset, block_length) = self.request_block(block_index)?;
            let data = self.receive_block(block_offset, block_length)?;
            self.buffer[block_offset..block_offset + block_length].copy_from_slice(&data);
        }
        Ok(self.buffer.clone())
    }

    fn request_block(&mut self, block_index: usize) -> io::Result<(usize, usize)> {
        let block_offset = block_index * self.block_length;
        let block_length = std::cmp::min(self.block_length, self.buffer.len() - block_offset);

        self.channel.request(block_offset, block_length)?;
        Ok((block_offset, block_length))
    }

    fn receive_block(&mut self, block_offset: usize, block_length: usize) -> io::Result<Vec<u8>> {
        let block = self.channel.receive()?;

        if block.offset != block_offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected block offset in response: expected {}, got {}",
                    block_offset, block.offset
                ),
            ));
        }
        if block.data.len() != block_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unexpected block data length in response: expected {}, got {}",
                    block_length,
                    block.data.len()
                ),
            ));
        }
        Ok(block.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_piece_downloader() {
        let channel = TestDownloadChannel::new(vec![]);
        let piece_downloader = PieceDownloader::new(channel, 10, 2);
        assert_eq!(vec![0; 10], piece_downloader.buffer);
    }

    #[test]
    fn test_download_piece() {
        let channel = TestDownloadChannel::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut piece_downloader = PieceDownloader::new(channel, 10, 3);

        let block_data = piece_downloader.download_piece().unwrap();
        assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], block_data);
    }

    #[test]
    #[should_panic]
    fn test_unexpected_offset_in_response() {
        let mut channel = TestDownloadChannel::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        channel.forced_offset = Some(1);
        let mut piece_downloader = PieceDownloader::new(channel, 10, 3);

        piece_downloader.download_piece().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_unexpected_data_length_in_response() {
        let mut channel = TestDownloadChannel::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        channel.forced_data_length = Some(1);
        let mut piece_downloader = PieceDownloader::new(channel, 10, 3);

        piece_downloader.download_piece().unwrap();
    }

    struct TestDownloadChannel {
        data: Vec<u8>,
        requested_block: Option<(usize, usize)>,
        forced_offset: Option<usize>,
        forced_data_length: Option<usize>,
    }

    impl TestDownloadChannel {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                requested_block: None,
                forced_offset: None,
                forced_data_length: None,
            }
        }
    }

    impl DownloadChannel for TestDownloadChannel {
        fn request(&mut self, offset: usize, length: usize) -> io::Result<()> {
            assert!(self.requested_block.is_none());
            self.requested_block = Some((offset, length));
            Ok(())
        }

        fn receive(&mut self) -> io::Result<Block> {
            if let Some((offset, length)) = self.requested_block {
                self.requested_block = None;

                let offset = self.forced_offset.unwrap_or(offset);
                let length = self.forced_data_length.unwrap_or(length);
                let data = self.data[offset..offset + length].to_vec();
                Ok(Block { offset, data })
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "No block requested"))
            }
        }
    }
}
