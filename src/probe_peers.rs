use std::net::SocketAddr;

pub fn probe_peers_sequential<R, E>(
    peer_addrs: &[SocketAddr],
    probe: impl Fn(&SocketAddr) -> Result<R, E>,
) -> Option<R> {
    for addr in peer_addrs {
        if let Ok(result) = probe(addr) {
            return Some(result);
        }
    }
    None
}
