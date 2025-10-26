use std::collections::HashMap;

use crate::bencoding::types::{BencValue, ByteString};

#[derive(Debug, PartialEq)]
pub struct Dict {
    values: HashMap<ByteString, BencValue>,
}

impl Dict {
    pub fn new(values: HashMap<ByteString, BencValue>) -> Self {
        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&BencValue> {
        self.values.get(&key.into())
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
