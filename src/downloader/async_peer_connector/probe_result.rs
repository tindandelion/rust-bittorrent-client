use std::io;

#[derive(Debug)]
pub enum ProbeError {
    InfoHashMismatch,
    IO(io::Error),
}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

pub type ProbeResult<T> = std::result::Result<T, ProbeError>;
