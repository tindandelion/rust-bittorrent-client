use sha1::Digest;

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct Sha1(Vec<u8>);

impl Sha1 {
    #[cfg(test)]
    pub fn new(value: Vec<u8>) -> Self {
        Self(value)
    }

    pub fn calculate(value: &[u8]) -> Self {
        let mut hasher = sha1::Sha1::new();
        hasher.update(value);
        Self(hasher.finalize().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}
