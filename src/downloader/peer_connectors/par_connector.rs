use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use mio::{Events, Poll, Token};

pub struct ParPeerConnector {
    timeout: Duration,
}

impl Default for ParPeerConnector {
    fn default() -> Self {
        Self {
            timeout: super::CONNECT_TIMEOUT,
        }
    }
}

impl ParPeerConnector {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn connect(
        self,
        peer_addrs: impl IntoIterator<Item = SocketAddr>,
    ) -> impl Iterator<Item = TcpStream> {
        PeerPoller::new(peer_addrs, self.timeout).expect("Failed to create peer iterator")
    }
}

struct PeerPoller {
    probes: HashMap<Token, PeerProbe>,
    poll: Poll,
    poll_timeout: Duration,
}

impl PeerPoller {
    fn new(
        peer_addrs: impl IntoIterator<Item = SocketAddr>,
        poll_timeout: Duration,
    ) -> IoResult<Self> {
        let mut probes: HashMap<Token, PeerProbe> = HashMap::new();
        let mut poll = Poll::new()?;

        for (index, addr) in peer_addrs.into_iter().enumerate() {
            let token = Token(index);
            let mut probe = PeerProbe::connect(token, addr)?;
            probe.register(&mut poll)?;
            probes.insert(token, probe);
        }

        Ok(Self {
            probes,
            poll,
            poll_timeout,
        })
    }

    fn wait_for_connect_event(&mut self) -> IoResult<Option<TcpStream>> {
        let mut events = mio::Events::with_capacity(1024);
        loop {
            if let Some(stream) = self.get_connected_stream()? {
                return Ok(Some(stream));
            }

            self.poll.poll(&mut events, Some(self.poll_timeout))?;
            if events.is_empty() {
                return Ok(None);
            }

            self.update_probe_states(&events);
            self.remove_errored_probes()?;
        }
    }

    fn remove_errored_probes(&mut self) -> IoResult<()> {
        for probe in self.probes.values_mut() {
            if probe.state == ProbeState::Error {
                probe.unregister(&mut self.poll)?;
            }
        }
        self.probes
            .retain(|_, probe| probe.state != ProbeState::Error);
        Ok(())
    }

    fn get_connected_stream(&mut self) -> IoResult<Option<TcpStream>> {
        let connected_probe_token = self
            .probes
            .iter()
            .find(|(_, probe)| probe.state == ProbeState::Connected)
            .map(|(token, _)| *token);

        if let Some(token) = connected_probe_token {
            let mut probe = self.probes.remove(&token).unwrap();
            probe.unregister(&mut self.poll)?;
            probe.into_std_tcp_stream().map(Some)
        } else {
            Ok(None)
        }
    }

    fn update_probe_states(&mut self, events: &Events) {
        for event in events.iter() {
            let token = event.token();
            let probe = self
                .probes
                .get_mut(&token)
                .expect(&format!("Unexpected token in received event: {token:?}"));
            probe.handle_connect_event();
        }
    }
}

type IoResult<T> = std::result::Result<T, std::io::Error>;

impl Iterator for PeerPoller {
    type Item = TcpStream;

    fn next(&mut self) -> Option<Self::Item> {
        self.wait_for_connect_event().unwrap_or_default()
    }
}

#[derive(PartialEq, Eq)]
enum ProbeState {
    Connecting,
    Connected,
    Error,
}

struct PeerProbe {
    token: Token,
    stream: mio::net::TcpStream,
    state: ProbeState,
}

impl PeerProbe {
    fn connect(token: Token, addr: SocketAddr) -> IoResult<Self> {
        let stream = mio::net::TcpStream::connect(addr)?;
        Ok(Self {
            token,
            stream,
            state: ProbeState::Connecting,
        })
    }

    fn register(&mut self, poll: &mut Poll) -> IoResult<()> {
        poll.registry()
            .register(&mut self.stream, self.token, mio::Interest::WRITABLE)?;
        Ok(())
    }

    fn handle_connect_event(&mut self) {
        self.state = match self.stream.peer_addr() {
            Ok(_) => ProbeState::Connected,
            Err(e) if e.kind() == std::io::ErrorKind::NotConnected => ProbeState::Error,
            Err(_) => ProbeState::Error,
        }
    }

    fn into_std_tcp_stream(self) -> IoResult<std::net::TcpStream> {
        let std_stream: std::net::TcpStream = self.stream.into();
        std_stream.set_nonblocking(false).map(|_| std_stream)
    }

    fn unregister(&mut self, poll: &mut Poll) -> IoResult<()> {
        poll.registry().deregister(&mut self.stream)
    }
}

#[cfg(test)]
mod tests {
    use crate::result::Result;
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn iterate_over_responsive_peers() -> Result<()> {
        let first_listener = TcpListener::bind("127.0.0.1:0")?;
        let second_listener = TcpListener::bind("127.0.0.1:0")?;
        let third_listener = TcpListener::bind("127.0.0.1:0")?;
        let mut responsive_addresses = vec![
            first_listener.local_addr()?,
            second_listener.local_addr()?,
            third_listener.local_addr()?,
        ];
        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?, // refuse to connect
            "192.0.2.1:6881".parse()?,  // timeout to connect
            responsive_addresses[0],    // responsive peer
            responsive_addresses[1],    // responsive peer
            responsive_addresses[2],    // responsive peer
        ];

        let connector = ParPeerConnector::default().with_timeout(Duration::from_secs(1));
        let mut connected_addresses = connector
            .connect(peer_addresses)
            .map(|stream| stream.peer_addr().unwrap())
            .collect::<Vec<_>>();

        connected_addresses.sort();
        responsive_addresses.sort();
        assert_eq!(connected_addresses, responsive_addresses);

        Ok(())
    }

    #[test]
    fn all_peers_are_unresponsive() -> Result<()> {
        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?, // refuse to connect
            "192.0.2.1:6881".parse()?,  // timeout to connect
        ];

        let connector = ParPeerConnector::default().with_timeout(Duration::from_secs(1));
        let connected_addresses = connector
            .connect(peer_addresses)
            .map(|stream| stream.peer_addr().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(connected_addresses, vec![]);

        Ok(())
    }
}
