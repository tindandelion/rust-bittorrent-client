mod decoder;
mod errors;
mod types;

use std::error::Error;

use crate::bencoding::decoder::Decoder;
pub use types::Dict;

pub fn decode_torrent_file(data: &[u8]) -> Result<Dict, Box<dyn Error>> {
    let mut decoder = Decoder::new(data);
    decoder.decode_dict().map_err(|e| e.into())
}
