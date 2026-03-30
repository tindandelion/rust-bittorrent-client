use std::{io, net::SocketAddr};

use bt_client::{
    downloader::peer_comm::HandshakeMessage,
    result::Result,
    types::{PeerId, Sha1},
};

mod test_env;
use test_env::TestEnv;

async fn exchange_handshake(
    addr: SocketAddr,
    peer_id: PeerId,
    info_hash: Sha1,
) -> Result<HandshakeMessage> {
    let mut channel = AsyncPeerChannel::connect(addr, peer_id).await?;
    channel.send_handshake(info_hash)?;
    let handshake = channel.read_handshake().await?;
    Ok(handshake)
}

#[test]
fn connect_to_peer() -> Result<()> {
    let env = TestEnv::start()?;
    let peer_id = PeerId::default();
    let torrent = TestEnv::read_torrent_file()?;

    let peer_address = env.get_peer_address()?;

    let fut = exchange_handshake(peer_address, peer_id, torrent.info.sha1);
    let handshake = runtime::block_on(fut)?;
    println!("received handshake: {handshake:?}");

    Ok(())
}

struct AsyncPeerChannel {
    peer_id: PeerId,
    stream: mio::net::TcpStream,
}

impl AsyncPeerChannel {
    pub async fn connect(addr: SocketAddr, peer_id: PeerId) -> io::Result<Self> {
        let stream = futures::ConnectFuture::new(addr).await?;
        Ok(Self { stream, peer_id })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn send_handshake(&mut self, info_hash: Sha1) -> io::Result<()> {
        let handshake = HandshakeMessage::new(info_hash, self.peer_id);
        handshake.send(&mut self.stream)
    }

    pub async fn read_handshake(&mut self) -> io::Result<HandshakeMessage> {
        let mut buffer = [0; HandshakeMessage::SIZE];
        futures::ReadExactFuture::new(&mut self.stream, &mut buffer).await?;
        HandshakeMessage::receive(&mut &buffer[..])
    }
}

mod futures {
    use io::Read;
    use std::{
        io,
        net::SocketAddr,
        pin::Pin,
        task::{Context, Poll},
    };

    pub struct ConnectFuture {
        addr: SocketAddr,
        stream: Option<mio::net::TcpStream>,
    }

    impl ConnectFuture {
        pub fn new(addr: SocketAddr) -> Self {
            Self { addr, stream: None }
        }
    }

    impl Future for ConnectFuture {
        type Output = io::Result<mio::net::TcpStream>;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.stream.is_none() {
                let stream = mio::net::TcpStream::connect(self.addr)?;
                self.stream = Some(stream);
            }

            let stream = self.stream.take().expect("the stream should be set");

            match stream.peer_addr() {
                Ok(_) => Poll::Ready(Ok(stream)),
                Err(err) if err.kind() == io::ErrorKind::NotConnected => {
                    self.stream = Some(stream);
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        }
    }

    pub struct ReadExactFuture<'a, 'b> {
        stream: &'a mut mio::net::TcpStream,
        buffer: &'b mut [u8],
    }

    impl<'a, 'b> ReadExactFuture<'a, 'b> {
        pub fn new(stream: &'a mut mio::net::TcpStream, buffer: &'b mut [u8]) -> Self {
            Self { stream, buffer }
        }
    }

    impl<'a, 'b> Future for ReadExactFuture<'a, 'b> {
        type Output = io::Result<()>;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut buf = vec![0; self.buffer.len()];
            let bytes_read = self.stream.read(&mut buf);
            match bytes_read {
                Ok(n) => {
                    if n == buf.len() {
                        self.buffer.copy_from_slice(&buf);
                        Poll::Ready(Ok(()))
                    } else {
                        Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Not enough data read",
                        )))
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
                Err(err) => Poll::Ready(Err(err)),
            }
        }
    }
}

mod runtime {
    use std::{
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    pub fn block_on<T>(future: impl Future<Output = T>) -> T {
        let waker = Waker::from(Arc::new(MyWaker));
        let mut context = Context::from_waker(&waker);
        let mut pinned = Box::pin(future);
        loop {
            match pinned.as_mut().poll(&mut context) {
                Poll::Pending => {
                    println!("Schedule other tasks");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                Poll::Ready(res) => break res,
            }
        }
    }

    struct MyWaker;

    impl Wake for MyWaker {
        fn wake(self: Arc<Self>) {}
    }
}
