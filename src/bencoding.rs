mod decoder;
pub mod error;
pub mod types;

use decoder::Decoder;

pub fn decode_dict(data: &[u8]) -> Result<types::Dict, error::Error> {
    let mut decoder = Decoder::new(data);
    decoder.decode_dict()
}
