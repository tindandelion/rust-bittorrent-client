mod decode_string;

pub fn bt_client() -> &'static str {
    let (string, _) = decode_string::decode_string("4:spam".as_bytes()).unwrap();
    string
}
