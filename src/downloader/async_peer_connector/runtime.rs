use std::{
    cell::{Cell, RefCell},
    io,
    time::Duration,
};

use mio::{Poll, Token};

pub struct Runtime {
    poll: RefCell<Poll>,
    next_id: Cell<usize>,
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().expect("Failed to create poll");
        Self {
            poll: RefCell::new(poll),
            next_id: Cell::new(0),
        }
    }

    pub fn next_token(&self) -> Token {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        Token(id)
    }

    fn register_stream(
        &self,
        stream: &mut mio::net::TcpStream,
        interests: mio::Interest,
    ) -> io::Result<Token> {
        let token = self.next_token();
        self.poll
            .borrow()
            .registry()
            .register(stream, token, interests)?;
        Ok(token)
    }

    fn deregister_stream(&self, stream: &mut mio::net::TcpStream) -> io::Result<()> {
        self.poll.borrow().registry().deregister(stream)
    }

    pub fn poll(&self, events: &mut mio::Events, timeout: Option<Duration>) -> io::Result<()> {
        self.poll.borrow_mut().poll(events, timeout)
    }
}

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

pub fn register_stream(
    stream: &mut mio::net::TcpStream,
    interests: mio::Interest,
) -> io::Result<Token> {
    RUNTIME.with(|rt| rt.register_stream(stream, interests))
}

pub fn deregister_stream(stream: &mut mio::net::TcpStream) -> io::Result<()> {
    RUNTIME.with(|rt| rt.deregister_stream(stream))
}

pub fn poll(events: &mut mio::Events, timeout: Option<Duration>) -> io::Result<()> {
    RUNTIME.with(|rt| rt.poll(events, timeout))
}
