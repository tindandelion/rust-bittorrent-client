use super::reactor;
use std::task::Waker;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

pub struct ReadExactFuture<'a, 'b, R>
where
    R: io::Read + mio::event::Source,
{
    id: Option<usize>,
    stream: &'a mut R,
    buffer: &'b mut [u8],
}

impl<'a, 'b, R> ReadExactFuture<'a, 'b, R>
where
    R: io::Read + mio::event::Source,
{
    pub fn new(stream: &'a mut R, buffer: &'b mut [u8]) -> Self {
        Self {
            id: None,
            stream,
            buffer,
        }
    }

    fn register(&mut self, waker: &Waker) -> io::Result<()> {
        if self.id.is_none() {
            let id = reactor::next_id();
            reactor::register_source(id, self.stream, mio::Interest::READABLE)?;
            reactor::set_waker(id, waker);
            self.id = Some(id);
        }
        Ok(())
    }

    fn deregister(&mut self) -> io::Result<()> {
        if let Some(id) = self.id {
            reactor::deregister_source(id, self.stream)?;
            self.id = None;
        }
        Ok(())
    }
}

impl<'a, 'b, R> Future for ReadExactFuture<'a, 'b, R>
where
    R: io::Read + mio::event::Source,
{
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.id.is_none() {
            self.register(cx.waker())?;
        }

        let id = self.id.expect("the id should be set");
        let mut buf = vec![0; self.buffer.len()];

        match self.stream.read(&mut buf) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                reactor::set_waker(id, cx.waker());
                Poll::Pending
            }
            Ok(n) => {
                self.deregister()?;
                if n == buf.len() {
                    self.buffer.copy_from_slice(&buf);
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!(
                            "Not enough data has been received: expected {}, received {}",
                            buf.len(),
                            n
                        ),
                    )))
                }
            }
            Err(err) => {
                self.deregister()?;
                Poll::Ready(Err(err))
            }
        }
    }
}

impl<'a, 'b, R> Drop for ReadExactFuture<'a, 'b, R>
where
    R: io::Read + mio::event::Source,
{
    fn drop(&mut self) {
        self.deregister().unwrap();
    }
}
