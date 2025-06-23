use std::collections::HashMap;

use crate::bencoding::types::{BencValue, Sha1};

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

    pub fn decode(&mut self) -> Result<BencValue, DecodeError> {
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

    pub fn decode_dict(&mut self) -> Result<Dict, DecodeError> {
        let mut values = HashMap::new();

        let dict_start = self.current_pos;
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            let key = self.decode_string()?;
            let value = self.decode()?;
            values.insert(key, value);
        }
        self.move_by(1);

        Ok(Dict::new(
            self.calculate_sha1(dict_start, self.current_pos),
            values,
        ))
    }

    #[cfg(test)]
    pub fn has_more_data(&self) -> bool {
        !self.rest_data.is_empty()
    }

    fn calculate_sha1(&self, start_index: usize, end_index: usize) -> Sha1 {
        Sha1::calculate(&self.raw_data[start_index..end_index])
    }

    fn decode_list(&mut self) -> Result<Vec<BencValue>, DecodeError> {
        let mut values: Vec<BencValue> = vec![];
        self.move_by(1);
        while !self.at_trailing_delimiter()? {
            let value = self.decode()?;
            values.push(value);
        }
        self.move_by(1);
        Ok(values)
    }

    fn decode_int(&mut self) -> Result<i64, DecodeError> {
        self.move_by(1);
        let end_index = self
            .rest_data
            .iter()
            .position(|&b| b == b'e')
            .ok_or(DecodeError::EndingDelimiterNotFound)?;

        let int_str = String::from_utf8_lossy(&self.rest_data[0..end_index]);
        self.move_by(end_index + 1);

        int_str
            .parse::<i64>()
            .map_err(|_| DecodeError::InvalidIntValue(int_str.to_string()))
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

    fn decode_string_length(&mut self) -> Result<usize, DecodeError> {
        let delimiter_index = self
            .rest_data
            .iter()
            .position(|&b| b == STRING_DELIMITER)
            .ok_or(DecodeError::StringDelimiterNotFound)?;

        let length_slice = &self.rest_data[0..delimiter_index];
        let length_str = String::from_utf8_lossy(length_slice);
        let string_length = length_str
            .parse::<usize>()
            .map_err(|_| DecodeError::InvalidStringLengthValue(length_str.to_string()))?;

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

        let decoded_value = decoder.decode().unwrap();
        assert_eq!(BencValue::from(""), decoded_value);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn non_empty_string() {
        let mut decoder = Decoder::new("4:spam".as_bytes());

        let decoded_value = decoder.decode().unwrap();
        assert_eq!(BencValue::from("spam"), decoded_value);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn ignore_trailing_bytes() {
        let mut decoder = Decoder::new("4:spam abcde".as_bytes());

        let decoded_value = decoder.decode().unwrap();
        assert_eq!(BencValue::from("spam"), decoded_value);
        assert_eq!(decoder.rest_data, " abcde".as_bytes());
    }

    #[test]
    fn represents_non_utf8_string() {
        let mut encoded = "6:spam".as_bytes().to_vec();
        encoded.extend_from_slice(&[0xF5, 0xF6]);
        let mut decoder = Decoder::new(&encoded);

        let decoded_value = decoder.decode().unwrap();
        assert_eq!(
            &encoded[2..],
            decoded_value.as_byte_string().unwrap().as_bytes()
        );
        assert!(!decoder.has_more_data());
    }

    #[cfg(test)]
    mod error_handling {
        use super::*;

        #[test]
        fn delimiter_not_found() {
            let mut decoder = Decoder::new("hello".as_bytes());

            assert_eq!(Err(DecodeError::StringDelimiterNotFound), decoder.decode());
        }

        #[test]
        fn non_numeric_length_value() {
            let mut decoder = Decoder::new("a:spam".as_bytes());

            assert_eq!(
                Err(DecodeError::InvalidStringLengthValue("a".to_string())),
                decoder.decode()
            );
        }

        #[test]
        fn negative_length_value() {
            let mut decoder = Decoder::new("-1:spam".as_bytes());

            assert_eq!(
                Err(DecodeError::InvalidStringLengthValue("-1".to_string())),
                decoder.decode(),
            );
        }

        #[test]
        fn string_length_is_invalid_utf_8_string() {
            let mut encoded = "1".as_bytes().to_vec();
            encoded.push(0xFF);
            encoded.extend_from_slice(":spam".as_bytes());
            let mut decoder = Decoder::new(&encoded);

            assert_eq!(
                Err(DecodeError::InvalidStringLengthValue("1�".to_string())),
                decoder.decode()
            );
        }

        #[test]
        fn length_is_too_big() {
            let mut decoder = Decoder::new("10:spam".as_bytes());

            assert_eq!(
                Err(DecodeError::StringLengthValueTooBig {
                    expected: 10,
                    actual: 4
                }),
                decoder.decode()
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

        let decoded = decoder.decode().unwrap();
        assert_eq!(BencValue::from(123456), decoded);
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn valid_negative_integer_value() {
        let encoded = "i-123456e".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded = decoder.decode().unwrap();
        assert_eq!(BencValue::from(-123456), decoded);
        assert!(!decoder.has_more_data());
    }

    #[cfg(test)]
    mod error_handling {
        use super::*;

        #[test]
        fn ending_delimiter_not_found() {
            let mut decoder = Decoder::new("i123456".as_bytes());

            assert_eq!(Err(DecodeError::EndingDelimiterNotFound), decoder.decode());
        }

        #[test]
        fn unable_to_parse_int() {
            let mut decoder = Decoder::new("iabce".as_bytes());

            assert_eq!(
                Err(DecodeError::InvalidIntValue("abc".to_string())),
                decoder.decode()
            );
        }

        #[test]
        fn invalid_utf8_string() {
            let mut decoder = Decoder::new(&[b'i', b'1', 0xFE, b'2', b'e']);

            assert_eq!(
                Err(DecodeError::InvalidIntValue("1�2".to_string())),
                decoder.decode()
            );
        }

        #[test]
        fn missing_content() {
            let mut decoder = Decoder::new("ie".as_bytes());

            assert_eq!(
                Err(DecodeError::InvalidIntValue("".to_string())),
                decoder.decode(),
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

        let decoded_value = decoder.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(0, decoded_dict.len());
        assert_eq!(&Sha1::calculate("de".as_bytes()), decoded_dict.sha1());
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn extracts_and_stores_string_values() {
        let encoded = "d3:cow3:moo4:spam4:eggse".as_bytes();
        let mut decoded = Decoder::new(encoded);

        let decoded_value = decoded.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(Some(&BencValue::from("moo")), decoded_dict.get("cow"));
        assert_eq!(Some(&BencValue::from("eggs")), decoded_dict.get("spam"));
        assert!(!decoded.has_more_data());
    }

    #[test]
    fn extracts_and_stores_dict_value_sha1_hashes() {
        let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(2, decoded_dict.len());
        assert_eq!(
            Some(&Sha1::calculate("d3:fooi1234ee".as_bytes())),
            decoded_dict.get_dict_sha1("spam")
        );
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn extracts_and_stores_integer_value() {
        let encoded = "d3:cow3:moo4:spami1234ee".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        assert_eq!(2, decoded_dict.len());
        assert_eq!(Some(&BencValue::from(1234)), decoded_dict.get("spam"));
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn extracts_and_stores_list_elements() {
        let encoded = "d4:spaml4:spam4:eggse3:cow3:mooe".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();
        let list_values: Vec<&ByteString> = decoded_dict
            .get("spam")
            .and_then(|x| x.as_list())
            .unwrap()
            .iter()
            .map(|x| x.as_byte_string().unwrap())
            .collect();
        assert_eq!(list_values, vec![&"spam".into(), &"eggs".into()]);
        assert!(decoder.rest_data.is_empty());
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
    fn empty_list() {
        let encoded = "le".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode().unwrap();
        let decoded_list = decoded_value.as_list().unwrap();
        assert_eq!(0, decoded_list.len());
        assert!(!decoder.has_more_data());
    }

    #[test]
    fn decode_list_of_strings() {
        let encoded = "l4:spam4:eggse".as_bytes();
        let mut decoder = Decoder::new(encoded);

        let decoded_value = decoder.decode().unwrap();
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

        assert_eq!(
            Err(DecodeError::EndingDelimiterNotFound),
            state.decode_list(),
        )
    }
}

#[cfg(test)]
mod complex_data_structures {
    use crate::bencoding::decoder::Decoder;

    #[test]
    #[ignore]
    fn decode_tracker_response() {
        let tracker_response = "d8:intervali900e5:peersld2:ip11:88.18.61.544:porti4666eed2:ip13:85.31.128.1114:porti52664eed2:ip13:95.58.175.2324:porti26163eed2:ip14:83.148.245.1864:porti51414eed2:ip14:15.204.231.2024:porti45548eed2:ip14:93.165.240.1044:porti56439eed2:ip14:193.148.16.2114:porti15981eed2:ip13:104.28.224.824:porti16570eed2:ip15:185.193.157.1874:porti25297eed2:ip14:37.120.185.2084:porti51413eed2:ip13:82.102.23.1394:porti39206eed2:ip14:92.101.157.2504:porti58130eed2:ip13:87.58.176.2384:porti62014eed2:ip13:87.58.176.2384:porti62004eed2:ip14:118.142.44.1464:porti6988eed2:ip10:95.33.0.764:porti22936eed2:ip13:73.196.29.1454:porti51413eed2:ip15:163.172.218.2154:porti31951eed2:ip13:63.210.25.1394:porti6886eed2:ip14:82.165.117.1884:porti1eed2:ip12:98.115.1.2084:porti50413eed2:ip15:109.226.251.1304:porti1230eed2:ip14:103.136.92.2524:porti14948eed2:ip14:193.32.127.2224:porti51765eed2:ip14:45.134.212.1014:porti46296eed2:ip13:82.65.230.1594:porti63812eed2:ip13:87.58.176.2384:porti62017eed2:ip13:189.46.193.814:porti9751eed2:ip14:217.174.206.674:porti51413eed2:ip14:183.107.103.254:porti51413eed2:ip13:81.201.16.2474:porti54694eed2:ip11:78.82.25.834:porti6887eed2:ip14:46.231.240.1874:porti50000eed2:ip12:134.3.183.424:porti58578eed2:ip13:73.81.101.1304:porti51414eed2:ip14:89.142.165.1314:porti51413eed2:ip13:82.24.182.2044:porti44346eed2:ip13:87.99.116.1484:porti51413eed2:ip13:87.58.176.2384:porti62015eed2:ip13:38.162.49.1954:porti6881eed2:ip13:82.64.112.1454:porti25561eed2:ip12:212.7.200.734:porti30151eed2:ip14:37.120.210.2114:porti9099eed2:ip12:37.112.5.2244:porti6881eed2:ip12:50.35.176.534:porti62904eed2:ip14:195.206.105.374:porti57402eed2:ip13:73.235.107.364:porti6881eed2:ip14:187.193.191.434:porti51765eed2:ip14:37.120.198.1724:porti12018eed2:ip14:185.21.216.1694:porti32774eeee";
        let mut decoder = Decoder::new(tracker_response.as_bytes());

        let decoded_value = decoder.decode().unwrap();
        let decoded_dict = decoded_value.as_dict().unwrap();

        let interval = decoded_dict.get("interval").unwrap().as_int().unwrap();
        assert_eq!(&900, interval);

        let peers = decoded_dict.get("peers").unwrap().as_list().unwrap();
        assert_eq!(peers.len(), 100);
    }
}
