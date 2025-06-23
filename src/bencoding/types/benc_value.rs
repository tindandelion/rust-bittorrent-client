use crate::bencoding::types::{ByteString, Dict};

#[derive(Debug, PartialEq)]
pub enum BencValue {
    Int(i64),
    ByteString(ByteString),
    Dict(Dict),
    List(Vec<BencValue>),
}

impl BencValue {
    pub fn as_int(&self) -> Option<&i64> {
        match self {
            BencValue::Int(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_byte_string(&self) -> Option<&ByteString> {
        match self {
            BencValue::ByteString(string) => Some(string),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            BencValue::Dict(dict) => Some(dict),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<BencValue>> {
        match self {
            BencValue::List(list) => Some(list),
            _ => None,
        }
    }
}

impl From<i64> for BencValue {
    fn from(value: i64) -> Self {
        BencValue::Int(value)
    }
}

impl From<&str> for BencValue {
    fn from(value: &str) -> Self {
        BencValue::ByteString(ByteString::from(value))
    }
}
