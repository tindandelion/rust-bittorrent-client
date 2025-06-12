use crate::types::ByteString;

#[derive(Debug, PartialEq)]
pub enum DecodeStringError {
    DelimiterNotFound,
    InvalidLengthValue {
        bytes: Vec<u8>,
        value: Option<String>,
    },
    LengthValueTooBig {
        expected: usize,
        actual: usize,
    },
}

impl std::error::Error for DecodeStringError {}

impl std::fmt::Display for DecodeStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn decode_string(encoded: &[u8]) -> Result<(ByteString, usize), DecodeStringError> {
    let delimiter_index = encoded
        .iter()
        .position(|&b| b == b':')
        .ok_or(DecodeStringError::DelimiterNotFound)?;

    let length_slice = &encoded[0..delimiter_index];
    let length_str = decode_string_length(length_slice)?;
    let string_length =
        length_str
            .parse::<usize>()
            .map_err(|_| DecodeStringError::InvalidLengthValue {
                bytes: length_slice.to_vec(),
                value: Some(length_str.to_string()),
            })?;

    let string_start = delimiter_index + 1;
    let string_end = string_start + string_length;

    if string_end > encoded.len() {
        return Err(DecodeStringError::LengthValueTooBig {
            expected: string_length,
            actual: encoded.len() - string_start,
        });
    }

    let string_bytes = &encoded[string_start..string_end];
    Ok((ByteString::new(string_bytes), string_end))
}

fn decode_string_length(slice: &[u8]) -> Result<&str, DecodeStringError> {
    str::from_utf8(slice).map_err(|_| DecodeStringError::InvalidLengthValue {
        bytes: slice.to_vec(),
        value: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        let encoded = "0:".as_bytes();

        let (decoded, consumed_length) = decode_string(encoded).unwrap();
        assert_eq!("", decoded.as_str().unwrap());
        assert_eq!(2, consumed_length);
    }

    #[test]
    fn non_empty_string() {
        let encoded = "4:spam".as_bytes();

        let (decoded, consumed_length) = decode_string(encoded).unwrap();
        assert_eq!("spam", decoded.as_str().unwrap());
        assert_eq!(6, consumed_length);
    }

    #[test]
    fn ignore_trailing_bytes() {
        let encoded = "4:spam abcde".as_bytes();

        let (decoded, consumed_length) = decode_string(encoded).unwrap();
        assert_eq!("spam", decoded.as_str().unwrap());
        assert_eq!(6, consumed_length);
    }

    #[test]
    fn represents_non_utf8_string() {
        let mut encoded = "6:spam".as_bytes().to_vec();
        encoded.extend_from_slice(&[0xF5, 0xF6]);

        let (decoded, consumed_length) = decode_string(&encoded).unwrap();
        assert_eq!(decoded.as_bytes(), &encoded[2..]);
        assert_eq!(consumed_length, 8);
    }
}

#[cfg(test)]
mod error_handling {
    use super::*;

    #[test]
    fn delimiter_not_found() {
        let encoded = "hello".as_bytes();
        assert_eq!(
            decode_string(encoded),
            Err(DecodeStringError::DelimiterNotFound)
        );
    }

    #[test]
    fn non_numeric_length_value() {
        let encoded = "a:spam".as_bytes();
        assert_eq!(
            decode_string(encoded),
            Err(DecodeStringError::InvalidLengthValue {
                bytes: vec![97],
                value: Some("a".to_string())
            })
        );
    }

    #[test]
    fn negative_length_value() {
        let encoded = "-1:spam".as_bytes();
        assert_eq!(
            decode_string(encoded),
            Err(DecodeStringError::InvalidLengthValue {
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

        assert_eq!(
            decode_string(&encoded),
            Err(DecodeStringError::InvalidLengthValue {
                bytes: vec![49, 0xFF],
                value: None,
            })
        );
    }

    #[test]
    fn length_is_too_big() {
        let encoded = "10:spam".as_bytes();
        assert_eq!(
            decode_string(encoded),
            Err(DecodeStringError::LengthValueTooBig {
                expected: 10,
                actual: 4
            })
        );
    }
}
