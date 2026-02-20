use std::{
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use tracing::{Level, instrument};

pub struct SeqPeerConnector<'a> {
    connect_timeout: Duration,
    progress_callback: Box<dyn Fn(SocketAddr, usize) + 'a>,
}

impl<'a> Default for SeqPeerConnector<'a> {
    fn default() -> Self {
        Self {
            connect_timeout: super::CONNECT_TIMEOUT,
            progress_callback: Box::new(|_, _| {}),
        }
    }
}

impl<'a> SeqPeerConnector<'a> {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
    pub fn with_progress_callback(
        mut self,
        progress_callback: impl Fn(SocketAddr, usize) + 'a,
    ) -> Self {
        self.progress_callback = Box::new(progress_callback);
        self
    }

    pub fn connect(
        self,
        addrs: impl IntoIterator<Item = SocketAddr>,
    ) -> impl Iterator<Item = TcpStream> {
        addrs
            .into_iter()
            .enumerate()
            .map(move |(index, addr)| {
                self.report_progress(addr, index);
                self.try_connect(addr)
            })
            .filter_map(Result::ok)
    }

    fn report_progress(&self, addr: SocketAddr, index: usize) {
        (self.progress_callback)(addr, index);
    }

    #[instrument(skip(self), ret, err, level = Level::DEBUG)]
    fn try_connect(&self, addr: SocketAddr) -> Result<TcpStream, std::io::Error> {
        TcpStream::connect_timeout(&addr, self.connect_timeout)
    }
}

#[cfg(test)]
mod tests {
    use crate::result::Result;
    use std::{cell::RefCell, net::TcpListener};

    use super::*;

    #[test]
    fn test_iterate_over_responsive_peers() -> Result<()> {
        let first_listener = TcpListener::bind("127.0.0.1:0")?;
        let second_listener = TcpListener::bind("127.0.0.1:0")?;

        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?,    // refuse to connect
            "192.0.2.1:6881".parse()?,     // timeout to connect
            first_listener.local_addr()?,  // responsive peer
            second_listener.local_addr()?, // responsive peer
        ];

        let connector = SeqPeerConnector::default().with_timeout(Duration::from_secs(1));
        let connected_addresses = connector
            .connect(peer_addresses)
            .map(|stream| stream.peer_addr().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            connected_addresses,
            vec![first_listener.local_addr()?, second_listener.local_addr()?,]
        );

        Ok(())
    }

    #[test]
    fn invoke_progress_callback_for_each_peer() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let peer_addresses = vec![
            "127.0.0.1:12345".parse()?, // refuse to connect
            "192.0.2.1:6881".parse()?,  // timeout to connect
            listener.local_addr()?,     // responsive peer
        ];
        let progress: RefCell<Vec<(SocketAddr, usize)>> = RefCell::new(vec![]);
        let progress_callback = |addr: SocketAddr, current: usize| {
            let mut curr = progress.borrow_mut();
            curr.push((addr, current));
        };

        let connector = SeqPeerConnector::default()
            .with_timeout(Duration::from_secs(1))
            .with_progress_callback(progress_callback);

        let iterator = connector.connect(peer_addresses.clone());
        let _ = iterator.collect::<Vec<_>>();

        assert_eq!(
            vec![
                (peer_addresses[0], 0),
                (peer_addresses[1], 1),
                (peer_addresses[2], 2),
            ],
            *progress.borrow()
        );

        Ok(())
    }
}
