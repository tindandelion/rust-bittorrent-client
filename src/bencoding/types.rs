use std::{collections::HashMap, str::Utf8Error};

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

#[derive(Debug, PartialEq)]
pub struct Dict {
    values: HashMap<ByteString, ByteString>,
}

impl Dict {
    pub fn new(values: HashMap<ByteString, ByteString>) -> Self {
        Self { values }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        value.as_str().ok()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
