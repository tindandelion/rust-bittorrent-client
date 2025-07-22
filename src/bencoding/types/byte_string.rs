use core::fmt;
use std::str::Utf8Error;

#[derive(PartialEq, Hash, Eq)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    pub fn new(value: &[u8]) -> Self {
        Self(value.to_vec())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.0)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
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
