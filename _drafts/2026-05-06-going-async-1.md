---
layout: post
title:  "Going async"
date: 2026-05-06 
---

In the [last post][last-post] I shared my experiences with programming non-blocking I/O on a low level, using Rust's [`mio`](https://docs.rs/mio/latest/mio/) library. It was a mixed bag: on one hand, non-blocking I/O was very useful to handle multiple TCP streams concurrently, but the overall programming experience and the resulting code structure left me wishing for something simpler. Starting from there, we're going to make a step forward towards a more ergonomic way to handle non-blocking I/O and dive deeper into _asynchronous programming_ in Rust. 

[*Version 0.1.4 on GitHub*][github-0.1.4]{: .no-github-icon}

Most of the work from this section is based on the knowledge I gathered from the book [_Asynchronous Programming in Rust_](https://www.goodreads.com/book/show/205552626-asynchronous-programming-in-rust) by Carl F. Samson. This is an excellent resource for those who want to learn the concepts of asynchronous programming, not only in Rust specifically. 

# Futures: basic building blocks 

A idea of a _future_ lies at the foundation of asynchronous programming. On the lowest level, a future is just a data type that implements the [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait with a single method [`Future::poll()`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll): 

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

Omitting some pesky implementation details, the core idea of a future is quite simple: it represents  a computation process whose result may not be immediately available, but we'll get to it eventually. While the computation is still in progress, the call to `poll()` returns immediately with `Poll::Pending`; once the computation is done, the call to `poll()` will return `Ready` with that computed value. In the programming jargon, we say that the future _resolves to a value of `T`_. 

For the caller, such an interface provides more flexibility. Instead of blocking the execution thread until the computation is done, the call to `poll()` returns immediately, which allows the caller decide what to do while the future is still pending. In the simplest case, the caller may decide to wait idly for a while and call `poll()` again. In a more useful scenario, the caller would choose to do some other work while the result isn't ready yet. 

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

### Compiler support: _async/await_

Let's see now how the compiler helps us escape the nightmare of writing the future implementations from the ground-up, with the help of `async/await` syntax. Spoiler alert: using `async/await` we can write the code in a much more direct and readable way. It looks almost like a regular synchronous code, with a sprinkle of magic here and there. 

Assuming that we have an asynchronous implementation of `TcpStream` type, the code for the `ReadTextFuture` we introduced above would look like this: 

```rust
pub async fn read_text(addr: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(addr).await?;

    let mut buf = vec![0_u8; 1024];
    stream.read_exact(&mut buf).await?;

    let text = String::from_utf8(buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(text)
}
```

Looks quite straightforward, right? This function reads almost like a normal synchronous function. Nonetheless, under the hood this is a state machine to orchestrate the futures, similar to what we talked above. Let's have a closer look. 

First, the `async` keyword in `async fn read_text(...)` means that calling this function does not execute its body immediately. Instead, it creates and returns a future object. As with other Rust futures, that future is lazy: real work starts only when someone polls it.

You'll notice also a few `.await` keywords here and there. Each `.await` is a _suspension point_ (or _yield point_) inside this future. In other words, these are the points where the generated `poll()` can interrupt its flow and return `Pending` to the caller. When called again, it behaves as if the execution gets resumed starting from that suspension point. There's no magic, though: that behaviour is guaranteed by the compiler carefully generating the state machine for us.  

# Async runtimes 

By now, we've talked in depth about what futures are and how to implement them, both directly via `Future` trait, and indirectly via async/await. However, we haven't touched yet on a very important topic: who actually drives futures to completion? Remember, futures by themselves are **inert**, so there's got to be someone to keep polling them to make progress. This is where the idea of an _async runtime_ enters the scene.

### What a runtime does 

The basic idea behind an async runtime is simple: given a future, keep calling its `poll()` method until the future resolves to a result. A naive implementation pops up into mind immediately: just keep calling `poll()` in a loop until it returns `Ready(T)`. It would work, but obviously that's going to be a very wasteful implementation that would just keep the CPU busy in that loop, while waiting for it completion. A more mature runtime should provide at least those capabilities:

* It should be able to run multiple futures concurrently; 
* It should schedule futures efficiently, so that ones that are not yet ready to progress don't waste CPU time. 

In particular, as our experience with `mio` illustrated, the OS provides us with the mechanisms to avoid active polling via I/O event queues. The async runtime should work in concert with these capabilities, to make sure that futures that are currently waiting for an I/O event don't get polled needlessly. On the other hand, once the I/O resource becomes ready, we'd like the runtime to poll that future as soon as possible. 

### Choose your runtime 

Interestingly, unlike other programming languages, Rust doesn't come with the "standard" async runtime. Instead, the runtimes are installed as separate crates. This decision, just like everything about software development, has both pros and cons. 

On a positive side, it gives developers a lot of flexibility on choosing an optimal runtime according to their project's needs and constraints. It also allows runtime implementations to evolve more quickly, because they are not constrained by the release cycle of Rust's standard library. 

On a negative side, it creates a bit of a mess in the async Rust ecosystem. Different runtimes are generally not interoperable, so mixing them can be awkward or impossible. For application developers, it's less of a problem: usually you just pick a single async runtime for your project and stick to it. For library developers, however, it's a much bigger pain. If you aim to develop a library with async features, you either need to pick a single runtime you're going to support, or go to the great lengths trying to make your library compatible with different runtimes. 

Today, by far the most popular async runtime option is `tokio`, with `smol` as a notable lightweight alternative:

* [`tokio`](https://docs.rs/tokio/latest/tokio/) - a general-purpose runtime with a rich set of async utilities and integrations.
* [`smol`](https://docs.rs/smol/latest/smol/) (and related ecosystem crates) - a modular approach that focuses on smaller building blocks.

There are also specialized runtimes for specific environments (for example, embedded or WebAssembly), but the key point stays the same: runtime choice is an explicit architectural decision in Rust.

### Core components of an async runtime

When we look at typical runtime components, we discover the following pieces:

* The _executor_ runs and schedules tasks (top-level futures), deciding what to poll next. Depending on design, it can be single-threaded or multi-threaded, with worker-local queues and task stealing between threads.
* The _reactor_ waits for OS events (socket readiness, timer expiration, etc.) and wakes the tasks that can now make progress.
* The runtime also typically provides async _resources_ such as timers, TCP streams, file wrappers, etc. These expose async APIs and integrate with the runtime so they can register interest in events and get woken up by the reactor.
* Async versions of _synchronization primitives_ such as channels, mutexes, semaphores, etc. 

The reactor and executor work together in a loosely coupled coordination: the executor provides each task with a [_Waker_](https://doc.rust-lang.org/std/task/struct.Waker.html), and when the reactor observes a ready event, it uses that waker to mark the task runnable again so the executor can poll it.

In practice, resources are usually tightly coupled to the runtime's reactor and wakeup machinery. That coupling is one of the main reasons interoperability between async runtimes in Rust is limited.

# Next steps 





[*Version 0.1.4 on GitHub*][github-0.1.4]{: .no-github-icon}

[last-post]: {{site.baseurl}}/{% post_url 2026-04-10-non-blocking-request-file %}
[github-0.1.4]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.1.4
