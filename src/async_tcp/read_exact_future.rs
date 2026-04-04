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
    bytes_read: usize,
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
            bytes_read: 0,
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
        let bytes_to_read = self.buffer.len() - self.bytes_read;
        let mut buf = vec![0; bytes_to_read];

        match self.stream.read(&mut buf) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                reactor::set_waker(id, cx.waker());
                Poll::Pending
            }
            Ok(0) => {
                self.deregister()?;
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Received zero bytes from stream",
                )))
            }
            Ok(n) => {
                let start_index = self.bytes_read;
                let end_index = start_index + n;
                let slice_to_fill = &mut self.buffer[start_index..end_index];

                slice_to_fill.copy_from_slice(&buf[..n]);
                self.bytes_read = end_index;

                if self.bytes_read == self.buffer.len() {
                    self.deregister()?;
                    Poll::Ready(Ok(()))
                } else {
                    reactor::set_waker(id, cx.waker());
                    Poll::Pending
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

#[cfg(test)]
mod tests {
    use crate::async_tcp::test_helpers::poll_future;

    use super::*;

    #[test]
    fn test_read_from_empty_stream() {
        let mut stream = Buffer::single_chunk(vec![]);
        let mut buffer: Vec<u8> = vec![0; 2];
        let future = ReadExactFuture::new(&mut stream, &mut buffer);

        let err = poll_future(future).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn test_read_exact_amount_in_single_poll() {
        let mut stream = Buffer::single_chunk(vec![42, 43, 44]);
        let mut buffer: Vec<u8> = vec![0; 2];
        let future = ReadExactFuture::new(&mut stream, &mut buffer);

        poll_future(future).unwrap();
        assert_eq!(buffer, vec![42, 43]);
    }

    #[test]
    fn test_read_exact_amount_in_multiple_polls() {
        let mut stream = Buffer::multiple_chunks(vec![vec![42, 43], vec![44]]);
        let mut buffer: Vec<u8> = vec![0; 3];
        let future = ReadExactFuture::new(&mut stream, &mut buffer);

        poll_future(future).unwrap();
        assert_eq!(buffer, vec![42, 43, 44]);
    }

    #[test]
    fn test_read_exact_amount_leaves_remaining_data_in_stream() {
        let mut stream = Buffer::multiple_chunks(vec![vec![42, 43], vec![44, 45]]);
        let mut buffer: Vec<u8> = vec![0; 3];
        let future = ReadExactFuture::new(&mut stream, &mut buffer);

        poll_future(future).unwrap();
        assert_eq!(buffer, vec![42, 43, 44]);
        assert_eq!(stream.0, vec![vec![45]]);
    }

    struct Buffer(Vec<Vec<u8>>);

    impl Buffer {
        fn single_chunk(data: Vec<u8>) -> Self {
            Self(vec![data])
        }

        fn multiple_chunks(data: Vec<Vec<u8>>) -> Buffer {
            Self(data)
        }
    }

    impl mio::event::Source for Buffer {
        fn register(
            &mut self,
            _registry: &mio::Registry,
            _token: mio::Token,
            _interests: mio::Interest,
        ) -> io::Result<()> {
            Ok(())
        }

        fn reregister(
            &mut self,
            _registry: &mio::Registry,
            _token: mio::Token,
            _interests: mio::Interest,
        ) -> io::Result<()> {
            Ok(())
        }

        fn deregister(&mut self, _registry: &mio::Registry) -> io::Result<()> {
            Ok(())
        }
    }

    impl io::Read for Buffer {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.is_empty() {
                return Ok(0);
            }

            let current_chunk = self.0.first_mut().unwrap();
            let bytes_to_read = std::cmp::min(buf.len(), current_chunk.len());

            let output = &mut buf[..bytes_to_read];
            output.copy_from_slice(&current_chunk[..bytes_to_read]);
            current_chunk.drain(..bytes_to_read);

            if current_chunk.is_empty() {
                self.0.remove(0);
            }

            Ok(bytes_to_read)
        }
    }
}
