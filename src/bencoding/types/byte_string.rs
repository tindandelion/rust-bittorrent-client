use core::fmt;

#[derive(PartialEq, Hash, Eq)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    pub fn new(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl From<&str> for ByteString {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl std::fmt::Display for ByteString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl fmt::Debug for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ByteString({})", self)
    }
}
