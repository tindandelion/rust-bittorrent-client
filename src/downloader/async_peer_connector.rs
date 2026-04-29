use super::PeerChannel;
use crate::{
    async_tcp,
    types::{PeerId, Sha1},
};
use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Duration,
};
use tracing::error;

mod connect_to_peer;
mod probe_result;
mod waker;

use probe_result::ProbeResult;
use waker::TaskWaker;

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
        PeerPoller::new(peer_addrs, self)
    }

    fn report_progress(&mut self, addr: SocketAddr) {
        self.peers_probed += 1;
        (self.progress_callback)(addr, self.peers_probed);
    }
}

struct PeerProbe {
    pub addr: SocketAddr,
    pub future: Pin<Box<dyn Future<Output = ProbeResult<PeerChannel>>>>,
}

struct PeerPoller<'a> {
    ready_queue: Arc<Mutex<Vec<usize>>>,
    pending_probes: HashMap<usize, PeerProbe>,
    connected_channels: Vec<PeerChannel>,
    connector: PeerConnector<'a>,
}

impl<'a> PeerPoller<'a> {
    fn new(peer_addrs: impl IntoIterator<Item = SocketAddr>, connector: PeerConnector<'a>) -> Self {
        let mut probes: HashMap<usize, PeerProbe> = HashMap::new();
        let mut ready_queue: Vec<usize> = vec![];

        for (id, addr) in peer_addrs.into_iter().enumerate() {
            let future = connect_to_peer::connect_to_peer(
                addr,
                connector.info_hash,
                connector.peer_id,
                connector.piece_count,
            );
            let probe = PeerProbe {
                addr,
                future: Box::pin(future),
            };
            ready_queue.push(id);
            probes.insert(id, probe);
        }

        Self {
            pending_probes: probes,
            connector,
            ready_queue: Arc::new(Mutex::new(ready_queue)),
            connected_channels: vec![],
        }
    }

    fn wait_for_connected_channel(&mut self) -> io::Result<Option<PeerChannel>> {
        loop {
            self.poll_ready_probes();

            if let Some(channel) = self.connected_channels.pop() {
                return Ok(Some(channel));
            }

            if self.pending_probes.is_empty() {
                return Ok(None);
            }

            if !async_tcp::poll_reactor(Some(self.connector.timeout))? {
                return Ok(None);
            }
        }
    }

    fn poll_ready_probes(&mut self) {
        let mut ready_probes: Vec<(usize, ProbeResult<PeerChannel>)> = vec![];
        {
            let mut ready_queue = self.ready_queue.lock().unwrap();
            while let Some(id) = ready_queue.pop() {
                let waker = Waker::from(Arc::new(TaskWaker::new(id, self.ready_queue.clone())));
                let mut context = Context::from_waker(&waker);
                let probe = self
                    .pending_probes
                    .get_mut(&id)
                    .unwrap_or_else(|| panic!("Unexpected id in received event: {id}"));

                if let Poll::Ready(res) = probe.future.as_mut().poll(&mut context) {
                    ready_probes.push((id, res));
                }
            }
        }

        for (id, result) in ready_probes {
            self.pending_probes.remove(&id).inspect(|probe| {
                self.connector.report_progress(probe.addr);
            });

            if let Ok(channel) = result {
                self.connected_channels.push(channel);
            }
        }
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
        let channel = connector
            .connect(vec![peer_addr])
            .next()
            .expect("failed to connect to peer");

        assert_eq!(peer_addr, channel.peer_addr());
        assert_eq!(remote_peer.peer_id(), channel.remote_id());
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
                send_bitfield_in_chunks(&mut stream, bitfield).unwrap();
                let msg = PeerMessage::receive(&mut stream).unwrap();
                if msg != PeerMessage::Interested {
                    panic!("expected interested message, received: {:?}", msg);
                }
                PeerMessage::Unchoke.send(&mut stream).unwrap();
            });
            peer_addr
        }
    }

    fn send_bitfield_in_chunks(stream: &mut impl io::Write, bitfield: Vec<u8>) -> io::Result<()> {
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
