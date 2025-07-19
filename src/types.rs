use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use sha1::Digest;
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerId([u8; 20]);

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub struct Sha1([u8; 20]);

pub struct Peer {
    pub ip: String,
    pub port: u16,
}

impl PeerId {
    #[cfg(test)]
    pub fn new(value: [u8; 20]) -> Self {
        Self(value)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl Sha1 {
    #[cfg(test)]
    pub fn new(value: [u8; 20]) -> Self {
        Self(value)
    }

    pub fn from_bytes(value: &[u8]) -> Self {
        Self(
            value
                .try_into()
                .expect(&format!("Invalid SHA-1 length: {}", value.len())),
        )
    }

    pub fn calculate(value: &[u8]) -> Self {
        let mut hasher = sha1::Sha1::new();
        hasher.update(value);
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Peer {
    pub fn to_socket_addr(&self) -> io::Result<SocketAddr> {
        (self.ip.as_str(), self.port)
            .to_socket_addrs()
            .map(|mut v| v.next().expect("Expected a single peer address"))
    }
}
