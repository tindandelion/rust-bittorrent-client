use std::collections::HashMap;

use crate::{bencoding::types::BencValue, types::Sha1};

use super::{
    error::Error,
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

    pub fn decode_dict(&mut self) -> Result<Dict, Error> {
        let mut values = HashMap::new();

        let dict_start = self.current_pos;
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            let key = self.decode_string()?;
            let value = self.decode_value()?;
            values.insert(key, value);
        }
        self.move_by(1);

        Ok(Dict::new(
            self.calculate_sha1(dict_start, self.current_pos),
            values,
        ))
    }

    fn decode_value(&mut self) -> Result<BencValue, Error> {
        match self.rest_data[0] {
            INT_START_DELIMITER => {
                let value = self.decode_int()?;
                Ok(BencValue::Int(value))
            }
            DICT_START_DELIMITER => {
                let value = self.decode_dict()?;
                Ok(BencValue::Dict(value))
            }
            LIST_START_DELIMITER => {
                let value = self.decode_list()?;
                Ok(BencValue::List(value))
            }
            _ => {
                let value = self.decode_string()?;
                Ok(BencValue::ByteString(value))
            }
        }
    }

    #[cfg(test)]
    pub fn has_more_data(&self) -> bool {
        !self.rest_data.is_empty()
    }

    fn calculate_sha1(&self, start_index: usize, end_index: usize) -> Sha1 {
        Sha1::calculate(&self.raw_data[start_index..end_index])
    }

    fn decode_list(&mut self) -> Result<Vec<BencValue>, Error> {
        let mut values: Vec<BencValue> = vec![];
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            let value = self.decode_value()?;
            values.push(value);
        }
        self.move_by(1);
        Ok(values)
    }

    fn decode_int(&mut self) -> Result<i64, Error> {
        self.move_by(1);
        let end_index = self
            .rest_data
            .iter()
            .position(|&b| b == b'e')
            .ok_or(Error::EndingDelimiterNotFound)?;

        let int_str = String::from_utf8_lossy(&self.rest_data[0..end_index]);
        self.move_by(end_index + 1);

        int_str
            .parse::<i64>()
            .map_err(|_| Error::InvalidIntValue(int_str.to_string()))
    }

    fn decode_string(&mut self) -> Result<ByteString, Error> {
        let string_length = self.decode_string_length()?;

        if string_length > self.rest_data.len() {
            return Err(Error::StringLengthValueTooBig {
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

    fn at_trailing_delimiter(&self) -> Result<bool, Error> {
        if self.rest_data.is_empty() {
            Err(Error::EndingDelimiterNotFound)
        } else {
            Ok(self.rest_data[0] == TRAILING_DELIMITER)
        }
    }

    fn decode_string_length(&mut self) -> Result<usize, Error> {
        let delimiter_index = self
            .rest_data
            .iter()
            .position(|&b| b == STRING_DELIMITER)
            .ok_or(Error::StringDelimiterNotFound)?;

        let length_slice = &self.rest_data[0..delimiter_index];
        let length_str = String::from_utf8_lossy(length_slice);
        let string_length = length_str
            .parse::<usize>()
            .map_err(|_| Error::InvalidStringLengthValue(length_str.to_string()))?;

        self.move_by(delimiter_index + 1);
        Ok(string_length)
    }
}

#[cfg(test)]
mod decode_string {
    use super::*;

    #[test]
    fn empty_string() {
        let mut decoder = Decoder::new("0:".as_bytes());

        let decoded_value = decoder.decode_value().unwrap();
        assert_eq!(BencValue::from(""), decoded_value);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn non_empty_string() {
        let mut decoder = Decoder::new("4:spam".as_bytes());

        let decoded_value = decoder.decode_value().unwrap();
        assert_eq!(BencValue::from("spam"), decoded_value);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn ignore_trailing_bytes() {
        let mut decoder = Decoder::new("4:spam abcde".as_bytes());

        let decoded_value = decoder.decode_value().unwrap();
        assert_eq!(BencValue::from("spam"), decoded_value);
        assert!(decoder.has_more_data());
    }

    #[test]
    fn represents_non_utf8_string() {
        let mut encoded = "6:spam".as_bytes().to_vec();
        encoded.extend_from_slice(&[0xF5, 0xF6]);
        let mut decoder = Decoder::new(&encoded);

        let decoded_value = decoder.decode_value().unwrap();
        assert_eq!(
            &encoded[2..],
            decoded_value.as_byte_string().unwrap().as_slice()
        );
        assert!(!decoder.has_more_data());
    }

    #[cfg(test)]
    mod error_handling {

        use super::*;

        #[test]
        fn delimiter_not_found() {
            let mut decoder = Decoder::new("hello".as_bytes());

            assert_eq!(Err(Error::StringDelimiterNotFound), decoder.decode_value());
        }

        #[test]
        fn non_numeric_length_value() {
            let mut decoder = Decoder::new("a:spam".as_bytes());

            assert_eq!(
                Err(Error::InvalidStringLengthValue("a".to_string())),
                decoder.decode_value()
            );
        }

        #[test]
        fn negative_length_value() {
            let mut decoder = Decoder::new("-1:spam".as_bytes());

            assert_eq!(
                Err(Error::InvalidStringLengthValue("-1".to_string())),
                decoder.decode_value(),
            );
        }

        #[test]
        fn string_length_is_invalid_utf_8_string() {
            let mut encoded = "1".as_bytes().to_vec();
            encoded.push(0xFF);
            encoded.extend_from_slice(":spam".as_bytes());
            let mut decoder = Decoder::new(&encoded);

            assert_eq!(
                Err(Error::InvalidStringLengthValue("1�".to_string())),
                decoder.decode_value()
            );
        }

        #[test]
        fn length_is_too_big() {
            let mut decoder = Decoder::new("10:spam".as_bytes());

            assert_eq!(
                Err(Error::StringLengthValueTooBig {
                    expected: 10,
                    actual: 4
                }),
                decoder.decode_value()
            );
        }
    }
}

#[cfg(test)]
mod decode_int {
    use super::*;

    #[test]
    fn valid_positive_integer_value() {
        let encoded = "i123456e".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded = decoder.decode_value().unwrap();
        assert_eq!(BencValue::from(123456), decoded);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn valid_negative_integer_value() {
        let encoded = "i-123456e".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded = decoder.decode_value().unwrap();
        assert_eq!(BencValue::from(-123456), decoded);
        assert!(!decoder.has_more_data());
    }

    #[cfg(test)]
    mod error_handling {
        use super::*;

        #[test]
        fn ending_delimiter_not_found() {
            let mut decoder = Decoder::new("i123456".as_bytes());

            assert_eq!(Err(Error::EndingDelimiterNotFound), decoder.decode_value());
        }

        #[test]
        fn unable_to_parse_int() {
            let mut decoder = Decoder::new("iabce".as_bytes());

            assert_eq!(
                Err(Error::InvalidIntValue("abc".to_string())),
                decoder.decode_value()
            );
        }

        #[test]
        fn invalid_utf8_string() {
            let mut decoder = Decoder::new(&[b'i', b'1', 0xFE, b'2', b'e']);

            assert_eq!(
                Err(Error::InvalidIntValue("1�2".to_string())),
                decoder.decode_value()
            );
        }

        #[test]
        fn missing_content() {
            let mut decoder = Decoder::new("ie".as_bytes());

            assert_eq!(
                Err(Error::InvalidIntValue("".to_string())),
                decoder.decode_value(),
            );
        }
    }
}

#[cfg(test)]
mod decode_dict {
    use super::*;

    #[test]
    fn empty_dict() {
        let mut decoder = Decoder::new("de".as_bytes());

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(0, decoded_dict.len());
        assert_eq!(&Sha1::calculate("de".as_bytes()), decoded_dict.sha1());
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn extracts_and_stores_string_values() {
        let encoded = "d3:cow3:moo4:spam4:eggse".as_bytes();
        let mut decoded = Decoder::new(encoded);

        let decoded_value = decoded.decode_value().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(Some(&BencValue::from("moo")), decoded_dict.get("cow"));
        assert_eq!(Some(&BencValue::from("eggs")), decoded_dict.get("spam"));
        assert!(!decoded.has_more_data());
    }

    #[test]
    fn extracts_and_stores_integer_elements() {
        let encoded = "d3:cow3:moo4:spami1234ee".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(2, decoded_dict.len());
        assert_eq!(Some(&BencValue::from(1234)), decoded_dict.get("spam"));
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn extracts_and_stores_dict_elements() {
        let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        let dict_field = decoded_dict.get("spam").unwrap().as_dict().unwrap();
        assert_eq!(Some(&BencValue::from(1234)), dict_field.get("foo"));
    }

    #[test]
    fn extracts_and_stores_list_elements() {
        let encoded = "d4:spaml4:spam4:eggse3:cow3:mooe".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();
        let list_values: Vec<&ByteString> = decoded_dict
            .get("spam")
            .and_then(|x| x.as_list())
            .unwrap()
            .iter()
            .map(|x| x.as_byte_string().unwrap())
            .collect();
        assert_eq!(list_values, vec![&"spam".into(), &"eggs".into()]);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn return_error_if_ending_delimiter_not_found() {
        let encoded = "d3:cow3:moo4:spam4:eggs".as_bytes();
        let mut state = Decoder::new(encoded);

        assert_eq!(state.decode_dict(), Err(Error::EndingDelimiterNotFound));
    }
}

#[cfg(test)]
mod decode_list {
    use super::*;

    #[test]
    fn empty_list() {
        let encoded = "le".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_list = decoded_value.as_list().unwrap();
        assert_eq!(0, decoded_list.len());
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn decode_list_of_strings() {
        let encoded = "l4:spam4:eggse".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode_value().unwrap();
        let decoded_list = decoded_value.as_list().unwrap();
        let list_values: Vec<&ByteString> = decoded_list
            .iter()
            .map(|x| x.as_byte_string().unwrap())
            .collect();
        assert_eq!(
            vec![&ByteString::from("spam"), &ByteString::from("eggs")],
            list_values
        );
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn returns_error_if_ending_delimiter_not_found() {
        let encoded = "l4:spam4:eggs".as_bytes();
        let mut state = Decoder::new(encoded);

        assert_eq!(Err(Error::EndingDelimiterNotFound), state.decode_value(),)
    }
}
