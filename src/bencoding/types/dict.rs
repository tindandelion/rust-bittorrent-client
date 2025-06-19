use std::collections::HashMap;

use crate::bencoding::types::{ByteString, Sha1};

#[derive(Debug, PartialEq)]
pub enum DictValue {
    String(ByteString),
    Dict(Dict),
}

#[derive(Debug, PartialEq)]
pub struct Dict {
    sha1: Sha1,
    values: HashMap<ByteString, DictValue>,
}

impl Dict {
    pub fn new(sha1: Sha1, values: HashMap<ByteString, DictValue>) -> Self {
        Self { sha1, values }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        match value {
            DictValue::String(string) => string.as_str().ok(),
            _ => None,
        }
    }

    pub fn get_dict_sha1(&self, key: &str) -> Option<&Sha1> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        match value {
            DictValue::Dict(dict) => Some(dict.sha1()),
            _ => None,
        }
    }

    pub fn sha1(&self) -> &Sha1 {
        &self.sha1
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
