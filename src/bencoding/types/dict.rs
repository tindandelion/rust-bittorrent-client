use std::collections::HashMap;

use crate::bencoding::types::{ByteString, Sha1};

#[derive(Debug, PartialEq)]
pub enum DictValue {
    ByteString(ByteString),
    Dict(Dict),
    Int(i64),
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
            DictValue::ByteString(string) => string.as_str().ok(),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<&i64> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        match value {
            DictValue::Int(value) => Some(value),
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
