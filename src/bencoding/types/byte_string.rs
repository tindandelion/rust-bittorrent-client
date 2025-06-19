use std::str::Utf8Error;

#[derive(Debug, PartialEq, Hash, Eq)]
pub struct ByteString {
    value: Vec<u8>,
}

impl ByteString {
    pub fn new(value: &[u8]) -> Self {
        Self {
            value: value.to_vec(),
        }
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.value)
    }

    #[cfg(test)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.value
    }
}
