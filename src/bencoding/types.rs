use std::{collections::HashMap, str::Utf8Error};

#[derive(Debug, PartialEq, Hash, Eq)]
pub struct ByteString {
    value: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum DictValue {
    String(ByteString),
}

#[derive(Debug, PartialEq)]
pub struct Dict {
    sha1: Vec<u8>,
    values: HashMap<ByteString, DictValue>,
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

impl Dict {
    pub fn new(sha1: Vec<u8>, values: HashMap<ByteString, DictValue>) -> Self {
        Self { sha1, values }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        match value {
            DictValue::String(string) => string.as_str().ok(),
        }
    }

    pub fn sha1(&self) -> &[u8] {
        &self.sha1
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
