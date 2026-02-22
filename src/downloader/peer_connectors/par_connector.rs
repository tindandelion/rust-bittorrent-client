use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use mio::{Events, Poll, Token};

pub struct ParPeerConnector<'a> {
    timeout: Duration,
    progress_callback: Box<dyn Fn(SocketAddr, usize) + 'a>,
    peers_probed: usize,
}

impl<'a> Default for ParPeerConnector<'a> {
    fn default() -> Self {
        Self {
            timeout: super::CONNECT_TIMEOUT,
            progress_callback: Box::new(|_, _| {}),
            peers_probed: 0,
        }
    }
}

impl<'a> ParPeerConnector<'a> {
    pub fn with_progress_callback(
        mut self,
        progress_callback: impl Fn(SocketAddr, usize) + 'a,
    ) -> Self {
        self.progress_callback = Box::new(progress_callback);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn connect(
        self,
        peer_addrs: impl IntoIterator<Item = SocketAddr>,
    ) -> impl Iterator<Item = TcpStream> {
        PeerPoller::new(peer_addrs, self.timeout, self.progress_callback)
            .expect("Failed to create peer iterator")
    }
}

struct PeerPoller<'a> {
    probes: HashMap<Token, PeerProbe>,
    poll: Poll,
    poll_timeout: Duration,
    peers_probed: usize,
    progress_callback: Box<dyn Fn(SocketAddr, usize) + 'a>,
}

impl<'a> PeerPoller<'a> {
    fn new(
        peer_addrs: impl IntoIterator<Item = SocketAddr>,
        poll_timeout: Duration,
        progress_callback: Box<dyn Fn(SocketAddr, usize) + 'a>,
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
            peers_probed: 0,
            progress_callback,
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
        let errored_tokens: Vec<Token> = self
            .probes
            .iter()
            .filter(|(_, probe)| probe.state == ProbeState::Error)
            .map(|(token, _)| *token)
            .collect();

        for token in errored_tokens {
            if let Some(mut probe) = self.probes.remove(&token) {
                self.unregister_probe(&mut probe)?;
            }
        }
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
            self.unregister_probe(&mut probe)?;

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

    fn unregister_probe(&mut self, probe: &mut PeerProbe) -> IoResult<()> {
        probe.unregister(&mut self.poll)?;
        self.peers_probed += 1;
        (self.progress_callback)(probe.addr, self.peers_probed);
        Ok(())
    }
}

type IoResult<T> = std::result::Result<T, std::io::Error>;

impl<'a> Iterator for PeerPoller<'a> {
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
    addr: SocketAddr,
}

impl PeerProbe {
    fn connect(token: Token, addr: SocketAddr) -> IoResult<Self> {
        let stream = mio::net::TcpStream::connect(addr)?;
        Ok(Self {
            token,
            stream,
            state: ProbeState::Connecting,
            addr,
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
    use std::{cell::RefCell, collections::HashSet, net::TcpListener};

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

    #[test]
    fn invoke_progress_callback_for_each_responsive_peer() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?, // refuse to connect
            "192.0.2.1:6881".parse()?,  // timeout to connect
            listener.local_addr()?,     // responsive peer
        ];
        let progress = RefCell::new(HashSet::<SocketAddr>::new());
        let progress_callback = |addr: SocketAddr, _: usize| {
            let mut curr = progress.borrow_mut();
            curr.insert(addr);
        };

        let connector = ParPeerConnector::default()
            .with_timeout(Duration::from_secs(1))
            .with_progress_callback(progress_callback);

        let iterator = connector.connect(peer_addresses.clone());
        let _ = iterator.collect::<Vec<_>>();

        assert_eq!(
            HashSet::from([peer_addresses[0], peer_addresses[2]]),
            *progress.borrow()
        );

        Ok(())
    }
}
