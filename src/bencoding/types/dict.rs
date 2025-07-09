use std::collections::HashMap;

use crate::{
    bencoding::types::{BencValue, ByteString},
    types::Sha1,
};

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

    pub fn sha1(&self) -> &Sha1 {
        &self.sha1
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
