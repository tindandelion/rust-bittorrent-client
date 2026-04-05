mod message_buffer;
mod mio_peer_stream;
mod probe_state;
use std::{collections::HashMap, io, net::SocketAddr, time::Duration};

use mio::{Events, Poll, Token, event::Event};
use tracing::{Span, debug, debug_span, error, trace, warn};

use bt_client::{
    downloader::{ PeerChannel},
    types::{PeerId, Sha1},
};

use mio_peer_stream::MioPeerStream;
use probe_state::{ProbeContext, ProbeError};

use probe_state::ProbeState;

pub struct PeerConnector<'a> {
    info_hash: Sha1,
    peer_id: PeerId,
    timeout: Duration,
    progress_callback: Box<dyn Fn(SocketAddr, usize) + 'a>,
    peers_probed: usize,
    piece_count: usize,
}

impl<'a> PeerConnector<'a> {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new(info_hash: Sha1, peer_id: PeerId, piece_count: usize) -> Self {
        Self {
            info_hash,
            peer_id,
            piece_count,
            timeout: Self::CONNECT_TIMEOUT,
            progress_callback: Box::new(|_, _| {}),
            peers_probed: 0,
        }
    }

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
    ) -> impl Iterator<Item = PeerChannel> {
        PeerPoller::new(peer_addrs, self).expect("Failed to create peer iterator")
    }

    fn report_progress(&mut self, addr: SocketAddr) {
        self.peers_probed += 1;
        (self.progress_callback)(addr, self.peers_probed);
    }
}

struct PeerPoller<'a> {
    probes: HashMap<Token, PeerProbe>,
    poll: Poll,
    connector: PeerConnector<'a>,
}

impl<'a> PeerPoller<'a> {
    fn new(
        peer_addrs: impl IntoIterator<Item = SocketAddr>,
        connector: PeerConnector<'a>,
    ) -> io::Result<Self> {
        let mut probes: HashMap<Token, PeerProbe> = HashMap::new();
        let mut poll = Poll::new()?;

        for (index, addr) in peer_addrs.into_iter().enumerate() {
            let token = Token(index);
            let context = ProbeContext {
                peer_id: connector.peer_id,
                info_hash: connector.info_hash,
                piece_count: connector.piece_count,
            };
            let mut probe = PeerProbe::connect(token, addr, context)?;
            probe.register(&mut poll)?;
            probes.insert(token, probe);
        }

        Ok(Self {
            probes,
            poll,
            connector,
        })
    }

    fn wait_for_connected_channel(&mut self) -> io::Result<Option<PeerChannel>> {
        let mut events = mio::Events::with_capacity(1024);
        loop {
            if let Some(channel) = self.get_connected_channel()? {
                return Ok(Some(channel));
            }

            self.poll.poll(&mut events, Some(self.connector.timeout))?;
            if events.is_empty() {
                return Ok(None);
            }

            self.update_probe_states(&events);
            self.remove_errored_probes()?;
            if self.probes.is_empty() {
                return Ok(None);
            }
        }
    }

    fn remove_errored_probes(&mut self) -> io::Result<()> {
        let errored_tokens: Vec<Token> = self
            .probes
            .iter()
            .filter(|(_, probe)| matches!(probe.state, ProbeState::Error))
            .map(|(token, _)| *token)
            .collect();

        for token in errored_tokens {
            if let Some(mut probe) = self.probes.remove(&token) {
                self.unregister_probe(&mut probe)?;
            }
        }
        Ok(())
    }

    fn get_connected_channel(&mut self) -> io::Result<Option<PeerChannel>> {
        let connected_probe_token = self
            .probes
            .iter()
            .find(|(_, probe)| probe.state.is_connected())
            .map(|(token, _)| *token);

        if let Some(token) = connected_probe_token {
            let mut probe = self.probes.remove(&token).unwrap();
            self.unregister_probe(&mut probe)?;

            probe.into_peer_channel().map(Some)
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
                .unwrap_or_else(|| panic!("Unexpected token in received event: {token:?}"));
            probe.handle_event(event);
        }
    }

    fn unregister_probe(&mut self, probe: &mut PeerProbe) -> io::Result<()> {
        probe.unregister(&mut self.poll)?;
        self.connector.report_progress(probe.addr);
        Ok(())
    }
}

impl<'a> Iterator for PeerPoller<'a> {
    type Item = PeerChannel;

    fn next(&mut self) -> Option<Self::Item> {
        self.wait_for_connected_channel()
            .inspect_err(|err| error!(%err, "error while processing I/O events"))
            .expect("error while processing I/O events")
    }
}

struct PeerProbe {
    token: Token,
    stream: MioPeerStream,
    state: ProbeState,
    addr: SocketAddr,
    span: Span,
}

impl PeerProbe {
    fn connect(token: Token, addr: SocketAddr, context: ProbeContext) -> io::Result<Self> {
        let span = debug_span!("connect_to_peer", addr = %addr);
        let stream = span.in_scope(|| {
            debug!("initiating connection");
            mio::net::TcpStream::connect(addr).map(MioPeerStream::new)
        })?;
        Ok(Self {
            token,
            stream,
            state: ProbeState::Connecting(context),
            addr,
            span,
        })
    }

    fn register(&mut self, poll: &mut Poll) -> io::Result<()> {
        poll.registry().register(
            &mut self.stream.inner,
            self.token,
            mio::Interest::WRITABLE | mio::Interest::READABLE,
        )?;
        Ok(())
    }

    fn handle_event(&mut self, event: &Event) {
        let _guard = self.span.enter();
        trace!(?event, "received event");

        if event.is_error() {
            match self.stream.inner.take_error() {
                Ok(Some(err)) => debug!(?err, "probe error: I/O error"),
                Ok(None) => {}
                Err(err) => debug!(?err, "failed to take I/O error"),
            }
            self.state = ProbeState::Error;
            return;
        }

        loop {
            match self.state.update(&mut self.stream) {
                Ok(next_state) => {
                    self.state = next_state;
                    if self.state.is_terminal() {
                        break;
                    }
                }
                Err(ProbeError::IO(err)) if err.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ProbeError::IO(err)) => {
                    debug!(?err, "probe error: I/O error");
                    self.state = ProbeState::Error;
                    break;
                }
                Err(err) => {
                    warn!(?err, "probe error");
                    self.state = ProbeState::Error;
                    break;
                }
            }
        }
    }

    fn into_peer_channel(self) -> io::Result<PeerChannel> {
        match self.state {
            ProbeState::Unchoked(remote_id) => {
                let std_stream: std::net::TcpStream = self.stream.inner.into();
                std_stream.set_nonblocking(false)?;

                PeerChannel::from_stream(std_stream, remote_id)
            }
            _ => Err(std::io::Error::other("peer did not unchoke")),
        }
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        poll.registry().deregister(&mut self.stream.inner)
    }
}

#[cfg(test)]
mod tests {
    use crate::downloader::peer_comm::{HandshakeMessage, PeerMessage};
    use crate::result::Result;
    use crate::types::{PeerId, Sha1};
    use std::{cell::RefCell, collections::HashSet, net::TcpListener};

    use super::*;

    const PIECE_COUNT: usize = 16;

    #[test]
    fn successful_handshake_with_remote_peer() {
        let remote_peer = TestRemotePeer::new();
        let peer_addr = remote_peer.start();

        let connector = make_connector();
        let channel = connector.connect(vec![peer_addr]).next().unwrap();

        assert_eq!(channel.peer_addr(), peer_addr);
        assert_eq!(channel.remote_id(), remote_peer.peer_id());
    }

    #[test]
    fn error_connect_refused() {
        let peer_addresses = vec!["127.0.0.1:12345".parse().unwrap()];

        let connector = make_connector();
        let connected_peers = connector.connect(peer_addresses).collect::<Vec<_>>();

        assert!(connected_peers.is_empty());
    }

    #[test]
    fn error_connect_timeout() {
        let peer_addresses = vec!["192.0.2.1:6881".parse().unwrap()];

        let connector = make_connector();
        let connected_peers = connector.connect(peer_addresses).collect::<Vec<_>>();

        assert!(connected_peers.is_empty());
    }

    #[test]
    fn error_handshake_hangup() {
        let remote_peer = TestRemotePeer::new().hangup_handshake();
        let peer_addr = remote_peer.start();

        let connector = make_connector();
        let connected_peers = connector.connect(vec![peer_addr]).collect::<Vec<_>>();

        assert!(connected_peers.is_empty());
    }

    #[test]
    fn iterate_over_responsive_peers() {
        let first_peer = TestRemotePeer::new();
        let second_peer = TestRemotePeer::new();
        let mut responsive_addresses = vec![first_peer.start(), second_peer.start()];
        let peer_addresses = vec![
            "127.0.0.1:12345".parse().unwrap(), // refuse to connect
            "192.0.2.1:6881".parse().unwrap(),  // timeout to connect
            responsive_addresses[0],            // responsive peer
            responsive_addresses[1],            // responsive peer
        ];

        let connector = make_connector().with_timeout(Duration::from_secs(1));
        let mut connected_addresses = connector
            .connect(peer_addresses)
            .map(|stream| stream.peer_addr())
            .collect::<Vec<_>>();

        connected_addresses.sort();
        responsive_addresses.sort();
        assert_eq!(connected_addresses, responsive_addresses);
    }

    #[test]
    fn all_peers_are_unresponsive() {
        let peer_addresses = vec![
            "127.0.0.1:12345".parse().unwrap(), // refuse to connect
            "192.0.2.1:6881".parse().unwrap(),  // timeout to connect
        ];

        let connector = make_connector().with_timeout(Duration::from_secs(1));
        let connected_peers = connector.connect(peer_addresses).collect::<Vec<_>>();

        assert!(connected_peers.is_empty());
    }

    #[test]
    fn invoke_progress_callback_for_each_peer() -> Result<()> {
        let remote_peer = TestRemotePeer::new();
        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?, // refuse to connect
            "192.0.2.1:6881".parse()?,  // timeout to connect
            remote_peer.start(),        // responsive peer
        ];
        let progress = RefCell::new(HashSet::<SocketAddr>::new());
        let progress_callback = |addr: SocketAddr, _: usize| {
            let mut curr = progress.borrow_mut();
            curr.insert(addr);
        };

        let connector = make_connector().with_progress_callback(progress_callback);

        let iterator = connector.connect(peer_addresses.clone());
        let _ = iterator.collect::<Vec<_>>();

        assert_eq!(
            HashSet::from([peer_addresses[0], peer_addresses[2]]),
            *progress.borrow()
        );

        Ok(())
    }

    fn make_connector<'a>() -> PeerConnector<'a> {
        PeerConnector::new(Sha1::random(), PeerId::random(), PIECE_COUNT)
            .with_timeout(Duration::from_secs(1))
    }

    struct TestRemotePeer {
        peer_id: PeerId,
        hangup_handshake: bool,
    }

    impl TestRemotePeer {
        pub fn new() -> Self {
            let peer_id = PeerId::random();
            Self {
                peer_id,
                hangup_handshake: false,
            }
        }

        fn hangup_handshake(mut self) -> Self {
            self.hangup_handshake = true;
            self
        }

        pub fn peer_id(&self) -> PeerId {
            self.peer_id
        }

        pub fn start(&self) -> SocketAddr {
            let listener =
                TcpListener::bind("127.0.0.1:0").expect("failed to start test peer listener");
            let peer_addr = listener
                .local_addr()
                .expect("failed to get local peer address");
            let peer_id = self.peer_id;
            let hangup_handshake = self.hangup_handshake;

            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                if hangup_handshake {
                    return;
                }

                let incoming_handshake = HandshakeMessage::receive(&mut stream).unwrap();
                let incoming_info_hash = incoming_handshake.info_hash;

                let handshake = HandshakeMessage::new(incoming_info_hash, peer_id);
                handshake.send(&mut stream).unwrap();
                let bitfield = vec![0b11111111, 0b11111111];
                send_bitfield_in_portions(&mut stream, bitfield).unwrap();
                let msg = PeerMessage::receive(&mut stream).unwrap();
                if msg != PeerMessage::Interested {
                    panic!("expected interested message, received: {:?}", msg);
                }
                PeerMessage::Unchoke.send(&mut stream).unwrap();
            });
            peer_addr
        }
    }

    fn send_bitfield_in_portions(stream: &mut impl io::Write, bitfield: Vec<u8>) -> io::Result<()> {
        let msg = PeerMessage::Bitfield(bitfield);
        let mut buffer = vec![];
        msg.send(&mut buffer)?;

        let msg_half = buffer.len() / 2;
        stream.write_all(&buffer[..msg_half])?;
        std::thread::sleep(Duration::from_millis(100));
        stream.write_all(&buffer[msg_half..])?;
        Ok(())
    }
}
