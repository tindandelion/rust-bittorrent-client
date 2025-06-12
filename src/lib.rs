pub fn bt_client() -> &'static str {
    "Hello, world!"
}

#[derive(Debug, PartialEq)]
enum DecodeStringError {
    DelimiterNotFound,
    InvalidLengthRepr(String),
    LengthValueTooBig {
        expected: usize,
        actual: usize,
    },
    InvalidEncodedString {
        encoded_bytes: Vec<u8>,
        invalid_pos: usize,
    },
}

impl std::error::Error for DecodeStringError {}

impl std::fmt::Display for DecodeStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn decode_string(encoded: &[u8]) -> Result<(&str, usize), DecodeStringError> {
    let delimiter_index = encoded
        .iter()
        .position(|&b| b == b':')
        .ok_or(DecodeStringError::DelimiterNotFound)?;

    let length_slice = &encoded[0..delimiter_index];
    let length_str = decode_utf8(length_slice)?;
    let string_length = length_str
        .parse::<usize>()
        .map_err(|_| DecodeStringError::InvalidLengthRepr(length_str.to_string()))?;

    let string_start = delimiter_index + 1;
    let string_end = string_start + string_length;

    if string_end > encoded.len() {
        return Err(DecodeStringError::LengthValueTooBig {
            expected: string_length,
            actual: encoded.len() - string_start,
        });
    }

    let string_bytes = &encoded[string_start..string_end];
    let string_content = decode_utf8(string_bytes)?;

    Ok((string_content, string_end))
}

fn decode_utf8(slice: &[u8]) -> Result<&str, DecodeStringError> {
    str::from_utf8(slice).map_err(|err| DecodeStringError::InvalidEncodedString {
        encoded_bytes: slice.to_vec(),
        invalid_pos: err.valid_up_to(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";
    const DICTIONARY_MARKER: u8 = b'd';

    use std::fs;

    #[test]
    fn read_torrent_file() {
        let contents = fs::read(TORRENT_FILE).unwrap();
        assert_eq!(contents[0], DICTIONARY_MARKER);

        let (first_string, consumed_length) = decode_string(&contents[1..]).unwrap();
        assert_eq!(first_string, "announce");

        let (second_string, _) = decode_string(&contents[(1 + consumed_length)..]).unwrap();
        assert_eq!(second_string, "http://bttracker.debian.org:6969/announce");
    }
}

#[cfg(test)]
mod decode_string {
    use super::*;

    #[test]
    fn empty_string() {
        let encoded = "0:".as_bytes();
        assert_eq!(decode_string(encoded), Ok(("", 2)));
    }

    #[test]
    fn non_empty_string() {
        let encoded = "4:spam".as_bytes();
        assert_eq!(decode_string(encoded), Ok(("spam", 6)));
    }

    #[test]
    fn ignore_trailing_bytes() {
        let encoded = "4:spam abcde".as_bytes();
        assert_eq!(decode_string(encoded), Ok(("spam", 6)));
    }

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
                Err(DecodeStringError::InvalidLengthRepr("a".to_string()))
            );
        }

        #[test]
        fn negative_length_value() {
            let encoded = "-1:spam".as_bytes();
            assert_eq!(
                decode_string(encoded),
                Err(DecodeStringError::InvalidLengthRepr("-1".to_string()))
            );
        }

        #[test]
        fn string_length_is_invalid_utf_8_string() {
            let mut encoded = "1".as_bytes().to_vec();
            encoded.push(0xFF);
            encoded.extend_from_slice(":spam".as_bytes());

            assert_eq!(
                decode_string(&encoded),
                Err(DecodeStringError::InvalidEncodedString {
                    encoded_bytes: encoded[..2].to_vec(),
                    invalid_pos: 1,
                })
            );
        }

        #[test]
        fn invalid_utf8_string_content() {
            let mut encoded = "5:spam".as_bytes().to_vec();
            encoded.push(0xff);

            assert_eq!(
                decode_string(&encoded),
                Err(DecodeStringError::InvalidEncodedString {
                    encoded_bytes: encoded[2..].to_vec(),
                    invalid_pos: 4,
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
}
