use std::net::SocketAddr;

pub fn probe_peers_sequential<R, E>(
    peer_addrs: &[SocketAddr],
    probe: impl Fn(&SocketAddr, usize) -> Result<R, E>,
) -> Option<R> {
    for (index, addr) in peer_addrs.iter().enumerate() {
        if let Ok(result) = probe(addr, index) {
            return Some(result);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::net::ToSocketAddrs;

    use super::*;
    type Result<T> = std::result::Result<T, String>;

    #[test]
    fn returns_result_of_first_successful_probe() {
        let peer_addrs = vec![
            localhost_with_port(12345),
            localhost_with_port(12346),
            localhost_with_port(12347),
        ];

        let result = probe_peers_sequential(&peer_addrs, |addr, _| {
            if addr.port() == 12347 {
                Result::Ok(*addr)
            } else {
                Result::Err("test error".to_string())
            }
        });

        assert_eq!(Some(peer_addrs[2]), result);
    }

    #[test]
    fn returns_none_if_all_probes_fail() {
        let peer_addrs = vec![
            localhost_with_port(12345),
            localhost_with_port(12346),
            localhost_with_port(12347),
        ];

        let result: Option<SocketAddr> =
            probe_peers_sequential(&peer_addrs, |_, _| Result::Err("test error".to_string()));

        assert_eq!(None, result);
    }

    fn localhost_with_port(port: u16) -> SocketAddr {
        ("127.0.0.1", port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    }
}
