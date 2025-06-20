use core::fmt;
use std::str::Utf8Error;

#[derive(PartialEq, Hash, Eq, Clone)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    pub fn new(value: &[u8]) -> Self {
        Self(value.to_vec())
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.0)
    }

    #[cfg(test)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for ByteString {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl ToString for ByteString {
    fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.0).to_string()
    }
}

impl fmt::Debug for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ByteString({})", self.to_string())
    }
}
