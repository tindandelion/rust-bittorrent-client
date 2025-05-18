pub fn bt_client() -> &'static str {
    "Hello, world!"
}

fn decode_string(encoded: &[u8]) -> (&str, usize) {
    let delimiter_index = encoded.iter().position(|&b| b == b':').unwrap();
    let string_length = str::from_utf8(&encoded[0..delimiter_index])
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let string_content =
        str::from_utf8(&encoded[delimiter_index + 1..delimiter_index + 1 + string_length]).unwrap();
    let consumed_length = delimiter_index + string_length + 1;

    (string_content, consumed_length)
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

        let (first_string, consumed_length) = decode_string(&contents[1..]);
        assert_eq!(first_string, "announce");

        let (second_string, _) = decode_string(&contents[(1 + consumed_length)..]);
        assert_eq!(second_string, "http://bttracker.debian.org:6969/announce");
    }
}

#[cfg(test)]
mod decode_string {
    use super::*;

    #[test]
    fn empty_string() {
        let encoded = "0:".as_bytes();
        assert_eq!(decode_string(encoded), ("", 2));
    }

    #[test]
    fn non_empty_string() {
        let encoded = "4:spam".as_bytes();
        assert_eq!(decode_string(encoded), ("spam", 6));
    }

    // TODO: Invalid strings
    // - Delimiter not found
    // - Negative length
    // - Non-numeric length
    // - Length is too large (end of buffer reached)
}
