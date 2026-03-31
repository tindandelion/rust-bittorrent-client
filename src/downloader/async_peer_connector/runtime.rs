use std::{
    cell::{Cell, RefCell},
    io,
    time::Duration,
};

use mio::{Poll, Token, event::Source};

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

    fn next_id(&self) -> Token {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        Token(id)
    }

    fn register_source(
        &self,
        stream: &mut impl Source,
        token: Token,
        interests: mio::Interest,
    ) -> io::Result<()> {
        self.poll
            .borrow()
            .registry()
            .register(stream, token, interests)
    }

    fn deregister_source(&self, stream: &mut impl Source) -> io::Result<()> {
        self.poll.borrow().registry().deregister(stream)
    }

    pub fn poll(&self, events: &mut mio::Events, timeout: Option<Duration>) -> io::Result<()> {
        self.poll.borrow_mut().poll(events, timeout)
    }
}

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

pub fn next_id() -> Token {
    RUNTIME.with(|rt| rt.next_id())
}

pub fn register_source(
    stream: &mut impl Source,
    token: Token,
    interests: mio::Interest,
) -> io::Result<()> {
    RUNTIME.with(|rt| rt.register_source(stream, token, interests))
}

pub fn deregister_source(stream: &mut impl Source) -> io::Result<()> {
    RUNTIME.with(|rt| rt.deregister_source(stream))
}

pub fn poll(events: &mut mio::Events, timeout: Option<Duration>) -> io::Result<()> {
    RUNTIME.with(|rt| rt.poll(events, timeout))
}
