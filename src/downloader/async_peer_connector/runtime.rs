use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io,
    time::Duration,
};

use mio::{Events, Poll, Token, event::Source};

pub struct Runtime {
    poll: RefCell<Poll>,
    events: RefCell<Events>,
    wakers: RefCell<HashMap<usize, std::task::Waker>>,
    next_id: Cell<usize>,
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().expect("Failed to create poll");
        let events = Events::with_capacity(1024);
        Self {
            poll: RefCell::new(poll),
            events: RefCell::new(events),
            next_id: Cell::new(0),
            wakers: RefCell::new(HashMap::new()),
        }
    }

    fn next_id(&self) -> usize {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        id
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

    fn deregister_source(&self, id: usize, stream: &mut impl Source) -> io::Result<()> {
        self.wakers.borrow_mut().remove(&id);
        self.poll.borrow().registry().deregister(stream)
    }

    pub fn poll(&self, timeout: Option<Duration>) -> io::Result<Vec<usize>> {
        let mut events = self.events.borrow_mut();
        self.poll.borrow_mut().poll(&mut events, timeout)?;
        let ids = events.iter().map(|event| event.token().0).collect();
        Ok(ids)
    }

    fn set_waker(&self, id: usize, waker: std::task::Waker) {
        self.wakers.borrow_mut().insert(id, waker);
    }
}

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

pub fn next_id() -> usize {
    RUNTIME.with(|rt| rt.next_id())
}

pub fn register_source(
    stream: &mut impl Source,
    id: usize,
    interests: mio::Interest,
) -> io::Result<()> {
    RUNTIME.with(|rt| rt.register_source(stream, Token(id), interests))
}

pub fn deregister_source(id: usize, stream: &mut impl Source) -> io::Result<()> {
    RUNTIME.with(|rt| rt.deregister_source(id, stream))
}

pub fn poll(timeout: Option<Duration>) -> io::Result<Vec<usize>> {
    RUNTIME.with(|rt| rt.poll(timeout))
}

pub(crate) fn set_waker(id: usize, waker: &std::task::Waker) {
    RUNTIME.with(|rt| rt.set_waker(id, waker.clone()))
}
