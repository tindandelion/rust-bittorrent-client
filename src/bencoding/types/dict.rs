use std::collections::HashMap;

use crate::bencoding::types::{BencValue, ByteString, Sha1};

#[derive(Debug, PartialEq)]
pub struct Dict {
    sha1: Sha1,
    values: HashMap<ByteString, BencValue>,
}

impl Dict {
    pub fn new(sha1: Sha1, values: HashMap<ByteString, BencValue>) -> Self {
        Self { sha1, values }
    }

    pub fn get(&self, key: &str) -> Option<&BencValue> {
        self.values.get(&key.into())
    }

    pub fn get_dict_sha1(&self, key: &str) -> Option<&Sha1> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        match value {
            BencValue::Dict(dict) => Some(dict.sha1()),
            _ => None,
        }
    }

    pub fn keys(&self) -> Vec<&ByteString> {
        self.values.keys().collect()
    }

    pub fn sha1(&self) -> &Sha1 {
        &self.sha1
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
