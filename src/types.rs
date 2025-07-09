use sha1::Digest;
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerId([u8; 20]);

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub struct Sha1([u8; 20]);

impl PeerId {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Sha1 {
    #[cfg(test)]
    pub fn new(value: [u8; 20]) -> Self {
        Self(value)
    }

    pub fn calculate(value: &[u8]) -> Self {
        let mut hasher = sha1::Sha1::new();
        hasher.update(value);
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}
