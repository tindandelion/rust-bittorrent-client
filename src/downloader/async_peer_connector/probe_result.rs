use std::io;

use crate::downloader::peer_comm::PeerMessage;

#[derive(Debug)]
pub enum ProbeError {
    InfoHashMismatch,
    BitfieldSizeMismatch,
    IncompleteFile,
    UnexpectedPeerMessage(#[allow(dead_code)] PeerMessage),
    IO(#[allow(dead_code)] io::Error),
}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

pub type ProbeResult<T> = std::result::Result<T, ProbeError>;
