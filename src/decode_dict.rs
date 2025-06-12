use std::{collections::HashMap, error::Error};

use crate::decode_string::{ByteString, decode_string};

pub struct Dict {
    values: HashMap<ByteString, ByteString>,
}

impl Dict {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        let key = ByteString::new(key.as_bytes());
        let value = self.values.get(&key)?;
        value.as_str().ok()
    }
}

struct DecoderState<'a> {
    data: &'a [u8],
    total_consumed_length: usize,
}

impl<'a> DecoderState<'a> {
    pub fn new(rest_data: &'a [u8]) -> Self {
        Self {
            data: rest_data,
            total_consumed_length: 0,
        }
    }

    fn move_by(&mut self, offset: usize) {
        self.data = &self.data[offset..];
        self.total_consumed_length += offset;
    }
}

pub fn decode_dict(encoded: &[u8]) -> Result<(Dict, usize), Box<dyn Error>> {
    let mut values = HashMap::new();
    let mut state = DecoderState::new(&encoded[1..]);

    while state.data[0] != b'e' {
        let (key, consumed_length) = decode_string(state.data)?;
        state.move_by(consumed_length);

        if state.data[0] == b'i' {
            let end_index = state.data.iter().position(|&b| b == b'e').unwrap();
            state.move_by(end_index + 1);
        } else if state.data[0] == b'd' {
            let (dict, consumed_length) = decode_dict(state.data)?;
            state.move_by(consumed_length);
        } else if state.data[0] == b'l' {
            decode_list(&mut state)?;
        } else {
            let (value, consumed_length) = decode_string(state.data)?;
            state.move_by(consumed_length);

            values.insert(key, value);
        }
    }

    return Ok((Dict { values }, state.total_consumed_length + 2));
}

fn decode_list(state: &mut DecoderState) -> Result<(), Box<dyn Error>> {
    state.move_by(1);
    while state.data[0] != b'e' {
        if state.data[0] == b'i' {
            let end_index = state.data.iter().position(|&b| b == b'e').unwrap();
            state.move_by(end_index + 1);
        } else if state.data[0] == b'd' {
            let (dict, consumed_length) = decode_dict(state.data)?;
            state.move_by(consumed_length);
        } else if state.data[0] == b'l' {
            let end_index = state.data.iter().position(|&b| b == b'e').unwrap();
            state.move_by(end_index + 1);
        } else {
            let (value, consumed_length) = decode_string(state.data)?;
            state.move_by(consumed_length);
        }
    }
    state.move_by(1);
    Ok(())
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

    #[test]
    fn skips_list_elements() {
        let encoded = "d4:spaml4:spam4:eggse3:cow3:mooe".as_bytes();
        let (decoded_dict, consumed_length) = decode_dict(encoded).unwrap();

        assert_eq!(1, decoded_dict.len());
        assert_eq!(consumed_length, encoded.len());
    }
}
