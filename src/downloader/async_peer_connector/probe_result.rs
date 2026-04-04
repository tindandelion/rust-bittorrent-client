use std::io;

pub enum ProbeError {
    IO(io::Error),
}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

pub type ProbeResult = std::result::Result<std::net::TcpStream, ProbeError>;
