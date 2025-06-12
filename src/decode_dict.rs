use std::{collections::HashMap, error::Error};

use crate::decode_string::{ByteString, decode_string};

pub struct Dict<'a> {
    values: HashMap<ByteString<'a>, ByteString<'a>>,
}

impl<'a> Dict<'a> {
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

pub fn decode_dict(encoded: &[u8]) -> Result<(Dict, usize), Box<dyn Error>> {
    let mut values = HashMap::new();
    let mut rest_data = &encoded[1..];
    let mut total_consumed_length = 1;

    while rest_data[0] != b'e' {
        let (key, consumed_length) = decode_string(rest_data)?;
        rest_data = &rest_data[consumed_length..];
        total_consumed_length += consumed_length;

        if rest_data[0] == b'i' {
            let end_index = rest_data.iter().position(|&b| b == b'e').unwrap();
            rest_data = &rest_data[end_index + 1..];
            total_consumed_length += end_index + 1;
        } else if rest_data[0] == b'd' {
            let (dict, consumed_length) = decode_dict(rest_data)?;
            rest_data = &rest_data[consumed_length..];
            total_consumed_length += consumed_length;
        } else {
            let (value, consumed_length) = decode_string(rest_data)?;
            rest_data = &rest_data[consumed_length..];
            total_consumed_length += consumed_length;

            values.insert(key, value);
        }
    }

    return Ok((Dict { values }, total_consumed_length + 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dict() {
        let encoded = "de".as_bytes();

        let (decoded_dict, consumed_length) = decode_dict(encoded).unwrap();

        assert_eq!(0, decoded_dict.len());
        assert_eq!(2, consumed_length);
    }

    #[test]
    fn dict_with_string_string_elements() {
        let encoded = "d3:cow3:moo4:spam4:eggse".as_bytes();

        let (decoded_dict, consumed_length) = decode_dict(encoded).unwrap();

        assert_eq!(2, decoded_dict.len());
        assert_eq!(consumed_length, encoded.len());
    }

    #[test]
    fn skips_integer_elements() {
        let encoded = "d3:cow3:moo4:spami1234ee".as_bytes();
        let (decoded_dict, consumed_length) = decode_dict(encoded).unwrap();

        assert_eq!(1, decoded_dict.len());
        assert_eq!(consumed_length, encoded.len());
    }

    #[test]
    fn skips_dict_elements() {
        let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
        let (decoded_dict, consumed_length) = decode_dict(encoded).unwrap();

        assert_eq!(1, decoded_dict.len());
        assert_eq!(consumed_length, encoded.len());
    }
}
