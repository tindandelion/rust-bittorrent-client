mod decoder;
pub mod errors;
pub mod types;

use decoder::Decoder;

pub fn decode_dict(data: &[u8]) -> Result<types::Dict, errors::DecodeError> {
    let mut decoder = Decoder::new(data);
    decoder.decode_dict()
}
