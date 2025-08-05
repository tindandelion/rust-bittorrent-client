use sha1::Digest;
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerId([u8; 20]);

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub struct Sha1([u8; 20]);

impl PeerId {
    pub fn new(value: [u8; 20]) -> Self {
        Self(value)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl Sha1 {
    #[cfg(test)]
    pub fn new(value: [u8; 20]) -> Self {
        Self(value)
    }

    pub fn from_bytes(value: &[u8]) -> Self {
        if value.len() != 20 {
            panic!("Invalid SHA-1 length: {}", value.len());
        }
        Self(value.try_into().unwrap())
    }

    pub fn calculate(value: &[u8]) -> Self {
        let mut hasher = sha1::Sha1::new();
        hasher.update(value);
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn verify(&self, data: &[u8]) -> bool {
        self == &Self::calculate(data)
    }
}
