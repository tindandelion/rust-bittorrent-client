#[derive(Debug, Clone, Copy, Default)]
pub struct PeerId([u8; 20]);

impl PeerId {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}
