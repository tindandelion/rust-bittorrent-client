mod decode_dict;
mod decode_string;

pub fn bt_client() -> &'static str {
    let (string, _) = decode_string::decode_string("4:spam".as_bytes()).unwrap();
    "Hello world"
}

#[cfg(test)]
mod tests {
    use super::*;

    const TORRENT_FILE: &str = "test-data/debian-12.11.0-amd64-netinst.iso.torrent";

    use std::fs;

    #[test]

    fn read_torrent_file() {
        let contents = fs::read(TORRENT_FILE).unwrap();

        let (first_dict, _) = decode_dict::decode_dict(&contents).unwrap();
    }
}
