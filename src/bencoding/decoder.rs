use std::collections::HashMap;

use crate::bencoding::types::{DictValue, Sha1};

use super::{
    errors::DecodeError,
    types::{ByteString, Dict},
};

pub struct Decoder<'a> {
    current_pos: usize,
    raw_data: &'a [u8],
    rest_data: &'a [u8],
}

const LIST_START_DELIMITER: u8 = b'l';
const DICT_START_DELIMITER: u8 = b'd';
const INT_START_DELIMITER: u8 = b'i';
const STRING_DELIMITER: u8 = b':';
const TRAILING_DELIMITER: u8 = b'e';

impl<'a> Decoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            current_pos: 0,
            raw_data: data,
            rest_data: data,
        }
    }

    pub fn decode_dict(&mut self) -> Result<Dict, DecodeError> {
        let mut values = HashMap::new();

        let dict_start = self.current_pos;
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            let key = self.decode_string()?;
            let value = self.decode_next_element()?;
            if let Some(value) = value {
                values.insert(key, value);
            }
        }
        self.move_by(1);

        Ok(Dict::new(
            self.calculate_sha1(dict_start, self.current_pos),
            values,
        ))
    }

    fn calculate_sha1(&self, start_index: usize, end_index: usize) -> Sha1 {
        Sha1::calculate(&self.raw_data[start_index..end_index])
    }

    fn decode_list(&mut self) -> Result<(), DecodeError> {
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            self.decode_next_element()?;
        }
        self.move_by(1);
        Ok(())
    }

    fn decode_int(&mut self) -> Result<(), DecodeError> {
        let end_index = self
            .rest_data
            .iter()
            .position(|&b| b == b'e')
            .ok_or(DecodeError::EndingDelimiterNotFound)?;
        self.move_by(end_index + 1);
        Ok(())
    }

    fn decode_string(&mut self) -> Result<ByteString, DecodeError> {
        let string_length = self.decode_string_length()?;

        if string_length > self.rest_data.len() {
            return Err(DecodeError::StringLengthValueTooBig {
                expected: string_length,
                actual: self.rest_data.len(),
            });
        }

        let string_bytes = &self.rest_data[..string_length];
        self.move_by(string_length);
        Ok(ByteString::new(string_bytes))
    }

    fn move_by(&mut self, offset: usize) {
        self.current_pos += offset;
        self.rest_data = &self.raw_data[self.current_pos..];
    }

    fn at_trailing_delimiter(&self) -> Result<bool, DecodeError> {
        if self.rest_data.is_empty() {
            Err(DecodeError::EndingDelimiterNotFound)
        } else {
            Ok(self.rest_data[0] == TRAILING_DELIMITER)
        }
    }

    fn decode_next_element(&mut self) -> Result<Option<DictValue>, DecodeError> {
        match self.rest_data[0] {
            INT_START_DELIMITER => {
                self.decode_int()?;
                Ok(None)
            }
            DICT_START_DELIMITER => {
                let value = self.decode_dict()?;
                Ok(Some(DictValue::Dict(value)))
            }
            LIST_START_DELIMITER => {
                self.decode_list()?;
                Ok(None)
            }
            _ => {
                let value = self.decode_string()?;
                Ok(Some(DictValue::ByteString(value)))
            }
        }
    }

    fn decode_string_length(&mut self) -> Result<usize, DecodeError> {
        let delimiter_index = self
            .rest_data
            .iter()
            .position(|&b| b == STRING_DELIMITER)
            .ok_or(DecodeError::StringDelimiterNotFound)?;

        let length_slice = &self.rest_data[0..delimiter_index];
        let length_str =
            str::from_utf8(length_slice).map_err(|_| DecodeError::InvalidStringLengthValue {
                bytes: length_slice.to_vec(),
                value: None,
            })?;
        let string_length =
            length_str
                .parse::<usize>()
                .map_err(|_| DecodeError::InvalidStringLengthValue {
                    bytes: length_slice.to_vec(),
                    value: Some(length_str.to_string()),
                })?;

        self.move_by(delimiter_index + 1);
        Ok(string_length)
    }
}

#[cfg(test)]
mod decode_string {
    use super::*;

    #[test]
    fn empty_string() {
        let encoded = "0:".as_bytes();
        let mut state = Decoder::new(encoded);

        let decoded = state.decode_string().unwrap();
        assert_eq!("", decoded.as_str().unwrap());
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn non_empty_string() {
        let encoded = "4:spam".as_bytes();
        let mut state = Decoder::new(encoded);

        let decoded = state.decode_string().unwrap();
        assert_eq!("spam", decoded.as_str().unwrap());
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn ignore_trailing_bytes() {
        let encoded = "4:spam abcde".as_bytes();
        let mut state = Decoder::new(encoded);

        let decoded = state.decode_string().unwrap();
        assert_eq!("spam", decoded.as_str().unwrap());
        assert_eq!(state.rest_data, " abcde".as_bytes());
    }

    #[test]
    fn represents_non_utf8_string() {
        let mut encoded = "6:spam".as_bytes().to_vec();
        encoded.extend_from_slice(&[0xF5, 0xF6]);

        let mut state = Decoder::new(&encoded);
        let decoded = state.decode_string().unwrap();
        assert_eq!(decoded.as_bytes(), &encoded[2..]);
        assert!(state.rest_data.is_empty());
    }

    #[cfg(test)]
    mod error_handling {
        use super::*;

        #[test]
        fn delimiter_not_found() {
            let encoded = "hello".as_bytes();
            let mut state = Decoder::new(encoded);

            assert_eq!(
                state.decode_string(),
                Err(DecodeError::StringDelimiterNotFound)
            );
        }

        #[test]
        fn non_numeric_length_value() {
            let encoded = "a:spam".as_bytes();
            let mut state = Decoder::new(encoded);

            assert_eq!(
                state.decode_string(),
                Err(DecodeError::InvalidStringLengthValue {
                    bytes: vec![97],
                    value: Some("a".to_string())
                })
            );
        }

        #[test]
        fn negative_length_value() {
            let encoded = "-1:spam".as_bytes();
            let mut state = Decoder::new(encoded);

            assert_eq!(
                state.decode_string(),
                Err(DecodeError::InvalidStringLengthValue {
                    bytes: vec![45, 49],
                    value: Some("-1".to_string())
                })
            );
        }

        #[test]
        fn string_length_is_invalid_utf_8_string() {
            let mut encoded = "1".as_bytes().to_vec();
            encoded.push(0xFF);
            encoded.extend_from_slice(":spam".as_bytes());

            let mut state = Decoder::new(&encoded);
            assert_eq!(
                state.decode_string(),
                Err(DecodeError::InvalidStringLengthValue {
                    bytes: vec![49, 0xFF],
                    value: None,
                })
            );
        }

        #[test]
        fn length_is_too_big() {
            let encoded = "10:spam".as_bytes();
            let mut state = Decoder::new(encoded);

            assert_eq!(
                state.decode_string(),
                Err(DecodeError::StringLengthValueTooBig {
                    expected: 10,
                    actual: 4
                })
            );
        }
    }
}

#[cfg(test)]
mod decode_int {
    use super::*;

    #[test]
    fn skip_value_and_move_past_integer_value() {
        let encoded = "i123456e".as_bytes();
        let mut state = Decoder::new(encoded);

        state.decode_int().unwrap();
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn return_error_if_ending_delimiter_not_found() {
        let encoded = "i123456".as_bytes();
        let mut state = Decoder::new(encoded);

        assert_eq!(
            state.decode_int(),
            Err(DecodeError::EndingDelimiterNotFound)
        );
    }
}

#[cfg(test)]
mod decode_dict {
    use super::*;

    #[test]
    fn empty_dict() {
        let mut state = Decoder::new("de".as_bytes());

        let decoded_dict = state.decode_dict().unwrap();

        assert_eq!(0, decoded_dict.len());
        assert_eq!(&Sha1::calculate("de".as_bytes()), decoded_dict.sha1());
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn extracts_and_stores_string_values() {
        let encoded = "d3:cow3:moo4:spam4:eggse".as_bytes();
        let mut state = Decoder::new(encoded);

        let decoded_dict = state.decode_dict().unwrap();

        assert_eq!(decoded_dict.get_string("cow"), Some("moo"));
        assert_eq!(decoded_dict.get_string("spam"), Some("eggs"));
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn extracts_and_stores_dict_value_sha1_hashes() {
        let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
        let mut state = Decoder::new(encoded);
        let decoded_dict = state.decode_dict().unwrap();

        assert_eq!(2, decoded_dict.len());
        assert_eq!(
            Some(&Sha1::calculate("d3:fooi1234ee".as_bytes())),
            decoded_dict.get_dict_sha1("spam")
        );
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn skips_integer_elements() {
        let encoded = "d3:cow3:moo4:spami1234ee".as_bytes();
        let mut state = Decoder::new(encoded);
        let decoded_dict = state.decode_dict().unwrap();

        assert_eq!(1, decoded_dict.len());
        assert_eq!(None, decoded_dict.get_string("spam"));
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn skips_list_elements() {
        let encoded = "d4:spaml4:spam4:eggse3:cow3:mooe".as_bytes();
        let mut state = Decoder::new(encoded);
        let decoded_dict = state.decode_dict().unwrap();

        assert_eq!(1, decoded_dict.len());
        assert_eq!(None, decoded_dict.get_string("spam"));
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn return_error_if_ending_delimiter_not_found() {
        let encoded = "d3:cow3:moo4:spam4:eggs".as_bytes();
        let mut state = Decoder::new(encoded);

        assert_eq!(
            state.decode_dict(),
            Err(DecodeError::EndingDelimiterNotFound)
        );
    }
}

#[cfg(test)]
mod decode_list {
    use super::*;

    #[test]
    fn skips_list_elements() {
        let encoded = "l4:spam4:eggse".as_bytes();
        let mut state = Decoder::new(encoded);

        state.decode_list().unwrap();
        assert!(state.rest_data.is_empty());
    }

    #[test]
    fn returns_error_if_ending_delimiter_not_found() {
        let encoded = "l4:spam4:eggs".as_bytes();
        let mut state = Decoder::new(encoded);

        assert_eq!(
            state.decode_list(),
            Err(DecodeError::EndingDelimiterNotFound)
        )
    }
}
