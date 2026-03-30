use std::{
    io,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use mio::{Poll, Token};

pub struct Runtime {
    poll: Mutex<Poll>,
    next_id: AtomicUsize,
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().expect("Failed to create poll");
        Self {
            poll: Mutex::new(poll),
            next_id: AtomicUsize::new(0),
        }
    }

    pub fn next_token(&self) -> Token {
        Token(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn register_stream(
        &self,
        stream: &mut mio::net::TcpStream,
        interests: mio::Interest,
    ) -> io::Result<Token> {
        let token = self.next_token();
        self.poll
            .lock()
            .expect("Failed to lock poll")
            .registry()
            .register(stream, token, interests)?;
        Ok(token)
    }

    pub fn deregister_stream(&self, stream: &mut mio::net::TcpStream) -> io::Result<()> {
        self.poll
            .lock()
            .expect("Failed to lock poll")
            .registry()
            .deregister(stream)
    }

    pub fn poll(&self, events: &mut mio::Events, timeout: Option<Duration>) -> io::Result<()> {
        self.poll
            .lock()
            .expect("Failed to lock poll")
            .poll(events, timeout)
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new())
}
