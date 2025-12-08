#[derive(Debug, Clone, Copy)]
pub struct FileInfo {
    pub file_length: usize,
    pub piece_length: u32,
}

impl FileInfo {
    pub fn piece_length(&self, piece_index: u32) -> u32 {
        let (piece_start, piece_end) = self.piece_bounds(piece_index);
        (piece_end - piece_start) as u32
    }

    pub fn piece_count(&self) -> u32 {
        self.file_length.div_ceil(self.piece_length as usize) as u32
    }

    pub fn piece_bounds(&self, piece_index: u32) -> (usize, usize) {
        let piece_start = piece_index as usize * self.piece_length as usize;
        let mut piece_end = (piece_index as usize + 1) * self.piece_length as usize;
        if piece_end > self.file_length {
            piece_end = self.file_length;
        };
        (piece_start, piece_end)
    }
}
