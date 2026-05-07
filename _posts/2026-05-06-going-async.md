---
layout: post
title:  "Going async"
date: 2026-05-06 
---

[*Version 0.1.4 on GitHub*][github-0.1.4]{: .no-github-icon}

In the [last post][last-post] I shared my experiences with programming non-blocking I/O on a low level, using Rust's [`mio`](https://docs.rs/mio/latest/mio/) library. It was a mixed bag: on one hand, non-blocking I/O was very useful to handle multiple TCP streams concurrently, but the overall programming experience and the resulting code structure left me wishing for something simpler. In this section, we're going to make a step forward towards a more ergonomic way to handle non-blocking I/O and dive deeper into _asynchronous programming_ in Rust. 

Most of the work from this section is based on the knowledge I got from a book [_Asynchronous Programming in Rust_](https://www.goodreads.com/book/show/205552626-asynchronous-programming-in-rust) by Carl F. Samson. This is an excellent resource for those who want to learn the concepts of asynchronous programming, not only in Rust specifically. 

# Futures: basic building blocks 

A idea of a _future_ lies at the foundation of asynchronous programming. On the lowest level, a future is just a data type that implements the `Future` trait with a single method `poll()`: 

```rust
pub enum Poll<T> {
    Pending,
    Ready(T),
}

pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

Omitting some pesky implementation details, the core idea of the future is quite simple: it represents  a computation process whose result may not be immediately available, but we'll get to it eventually. While the computation is still in progress, the call to `poll()` returns immediately with `Poll::Pending`; once the computation is done, the call to `poll()` will return `Ready` with that computed value. In the programming jargon, we say that the future _resolves to a value of `T`_. 

For the caller, this interface provides more flexibility. Instead of blocking the execution thread until the computation is done, the call to `poll()` returns immediately, which allows the caller decide what to do while the future is still pending. In the simplest case, the caller may decide to wait idly for a while and call `poll()` again. In a more useful scenario, the caller would choose to do some other work while the result isn't ready yet. 

The overall idea is that a call to `poll()` never blocks the caller's execution flow for a long time. Behind this simple interface, however, can hide a complex implementation to ensure that each call to `poll()` returns quickly. 

Consider, for example, the calculation of n-th Fibonacci number: a typical example of a potentially long-running computation. To avoid blocking the caller for a long time, the implementation could instead do calculations in steps: the first call to `poll()` could only calculate first 100 numbers, store the intermediate results, and return `Pending`. The second call would pick up where the last call stopped, and calculate next 100 numbers. The caller would call `poll()` in a loop until eventually the future resolves to n-th number in the sequence. Effectively, inside this data type we implement a state machine that advances towards a result with each call to `poll()`. 

Another example from a non-blocking I/O realm could be connecting to the TCP stream. As we know from before, `mio`'s implementation of `TcpStream::connect()` is non-blocking, so we need to wait for a while until the TCP socket becomes ready to transmit the data. This can also be thought of as a future type `TcpConnectFuture`. Its `poll()` method would check the underlying TCP socket, returning `Pending` while the socket is not yet readable. Once it's ready, the call to `poll()` would return `Ready(TcpStream)`, and we can use the returned stream to send data. 

### Futures in Rust are lazy

Let's have a second look at the `TcpConnectFuture` from above. The question becomes: when do we actually call `TcpStream::connect()`? There's two possible places to do that: 

* Immediately in the constructor of `TcpConnectFuture`; 
* Deferred, at the first call to `Future::poll()`; 

If we do it in the constructor, our future is _eager:_ the work starts immediately. If, on the other hand, we defer the work until the first call to `Future::poll()`, our future is said to be _lazy_. Both approaches have pros and cons, there's no clean-cut answer which one is better. 

In Rust, specifically, futures **are supposed to be lazy**: no work should be started until the explicit call to `Future::poll()`. The benefit of laziness is that the caller controls when and how work runs, which is essential when you might be on a microcontroller with no heap, or building a custom scheduler.

### Leaf vs non-leaf futures

When reasoning about futures, there is an interesting and useful distinction to make between _leaf_ and _non-leaf_ futures. 

A **leaf future** is a future that talks directly to some external async source, such as a socket, timer, or file descriptor. In other words, at the bottom of the chain there is a concrete operation that can be pending in the real world. For example, we could have a `ConnectFuture` to connect to a TCP socket, and `ReadExactFuture(stream, n_bytes)` to asynchronously read N bytes from the socket. 

When implementing leaf futures, you usually can't avoid implementing the `Future` trait manually, to do all pesky low-level details of managing the underlying resource.

In contrast, a **non-leaf future** is a future that does not perform a low-level operation itself. Instead, it orchestrates other futures: it polls them, combines their results, and decides what to do next. In other words, it operates on a higher level of abstraction, and at that level we can get help from the compiler to make coding more ergonomic.

Let's illustrate that with a small example. Suppose we want to build on top of `ConnectFuture` and `ReadExactFuture` a new piece of functionality, called `ReadTextFuture(addr)` that should:

* Connect to a TCP stream by address;
* Read 1024 bytes from that stream;
* Convert those bytes into a `String` and return it.

Our new `ReadTextFuture` combines `ConnectFuture` and `ReadExactFuture` in a new interesting way. Being itself a future, it must implement that in `Future::poll()`, which is basically a simple state machine:

[Picture]

Generally speaking, `ReadTextFuture` maintains the current state of the operation and polls the lower-level futures. If a lower-level future is still pending, `ReadTextFuture` is also pending. When the call to `poll()` of a lower-level future ends with a result, `ReadTextFuture` can advance to the next step.

Having looked at what it takes to implement an operation as simple as `ReadTextFuture` from scratch, a question comes to mind:

> Who on earth would like to program like that? <br/>Even the simplest task results in writing tedious amounts of code!

And that's true, if we had to program like that, it would be a nightmare of a job. And that's where the distinction between leaf and non-leaf futures becomes important.

You see, this pattern "wait until a lower-level future completes, then progress to the next step" is so mechanical and generic, that we can leave it to the compiler to generate the implementation of all nitty-gritty details of `Future::poll()` and never have to do it manually! Instead, we can focus on what's really important: the algorithm the future implements.

# Compiler support: _async/await_

# Async runtime from the ground up

# Next steps 





[*Version 0.1.4 on GitHub*][github-0.1.4]{: .no-github-icon}

[last-post]: {{site.baseurl}}/{% post_url 2026-04-10-non-blocking-request-file %}
[github-0.1.4]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.1.4
