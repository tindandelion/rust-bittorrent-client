---
layout: post
title:  "Intro to async Rust"
date: 2026-05-06 
---

In the [last post][last-post] I shared my experiences with programming non-blocking I/O at a fairly low level, using Rust's [`mio`](https://docs.rs/mio/latest/mio/) library. That experience was a mixed bag: non-blocking I/O was very useful for handling multiple TCP streams concurrently, but the overall programming experience and the resulting code structure [left me wishing for something simpler][last-post-reflections]. Motivated by this frustration, we're going to take a step forward toward a more ergonomic way to handle non-blocking I/O, and dive deeper into _asynchronous programming_ in Rust.

This post is a brief introduction to async Rust. We'll do real coding as the [next step](#whats-next).

# Futures: basic building blocks

The idea of a _future_ lies at the foundation of asynchronous programming. At the lowest level, a future is just a data type that implements the [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait with a single method, [`Future::poll()`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll):

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

Omitting some pesky implementation details, the core idea of a future is quite simple: it represents a computation process whose result may not be immediately available, but we'll get it eventually. While the computation is still in progress, the call to `poll()` returns immediately with `Poll::Pending`; once the computation is done, the call to `poll()` returns `Ready` with the computation result. In programming jargon, we say that the future _resolves to a value of `T`_.

For the caller, such an interface provides more flexibility. Instead of blocking the execution flow until the computation is done, the call to `poll()` returns early, which gives the caller an opportunity to decide what to do while the future is still pending. In the simplest case, the caller may decide to wait idly for a while and call `poll()` again. In a more useful scenario, the caller can do other work while the result isn't ready yet.

The overall idea is that a call to `poll()` never blocks the caller's execution flow for a long time. Behind this simple interface, however, there may be a complex implementation ensuring that each call to `poll()` returns quickly.

Consider, for example, the calculation of the n-th Fibonacci number: a typical example of a potentially long-running computation. To avoid blocking the caller for a long time, the implementation could instead do calculations in steps: the first call to `poll()` could only calculate the first 100 numbers, store the intermediate results internally, and return `Pending`. The second call would pick up where the last call stopped and calculate the next 100 numbers. The caller would keep calling `poll()` until eventually the future resolves to the final result. Effectively, inside this data type we implement a state machine that advances toward a result with each call to `poll()`.

Another example from the realm of non-blocking I/O could be connecting to a TCP stream. As we know from before, `mio`'s implementation of `TcpStream::connect()` is non-blocking, so we need to wait until the TCP socket becomes ready to transmit data. This can also be thought of as a future type, `TcpConnectFuture`. Its `poll()` method would check the underlying TCP socket, returning `Pending` while the socket is not yet ready. Once it's ready, the call to `poll()` would return `Ready(TcpStream)`, and we can use the returned stream to send data.

### Futures in Rust are lazy

Let's take a second look at the `TcpConnectFuture` from above. The question becomes: when do we actually call `TcpStream::connect()`? There are two possible places to do that:

* Immediately in the constructor of `TcpConnectFuture`;
* Deferred, at the first call to `Future::poll()`;

If we do it in the constructor, our future is _eager:_ the work starts immediately. If, on the other hand, we defer the work until the first call to `Future::poll()`, our future is said to be _lazy_. Both approaches have pros and cons; there's no clear-cut answer to which one is better.

In Rust, specifically, futures **are supposed to be lazy**: no work should be started until the explicit call to `Future::poll()`. The benefit of laziness is that the caller controls when and how work runs, which is essential when you might be on a microcontroller with no heap, or building a custom scheduler.

### Leaf vs non-leaf futures

When reasoning about futures, there is an interesting and useful distinction to make between _leaf_ and _non-leaf_ futures.

A **leaf future** is a future that talks directly to some external async source, such as a socket, timer, or file descriptor. In other words, at the bottom of the chain there is a concrete operation that can be pending in the real world. For example, we could have a `ConnectFuture` to connect to a TCP socket, and `ReadExactFuture(stream, n_bytes)` to asynchronously read N bytes from the socket.

When developing leaf futures, you usually can't avoid implementing the `Future` trait manually to handle all the pesky low-level details of managing the underlying resource.

In contrast, a **non-leaf future** is a future that does not perform a low-level operation itself. Instead, it orchestrates other futures: it polls them, combines their results, and decides what to do next. In other words, it operates on a higher level of abstraction.

Let's illustrate that with a small example. Suppose we want to build a new piece of functionality on top of `ConnectFuture` and `ReadExactFuture`, called `ReadTextFuture(addr)`, that should:

* Connect to a TCP stream by address;
* Read 1024 bytes from that stream;
* Convert those bytes into a `String` and return it.

Our new `ReadTextFuture` combines `ConnectFuture` and `ReadExactFuture` in a new interesting way. Being itself a future, it must implement that in `Future::poll()`, which is basically a simple state machine:

![ReadTextFuture state diagram]({{ site.baseurl }}/assets/images/intro-to-async-rust/read-text-future.svg)

Generally speaking, `ReadTextFuture` maintains the current state of the operation and polls the lower-level futures. If a lower-level future is still pending, `ReadTextFuture` is also pending. When the call to `poll()` of a lower-level future ends with a result, `ReadTextFuture` can advance to the next step.

Okay, so we're back to [implementing state machines][last-post-state-machine], even for an operation as simple as `ReadTextFuture`, from scratch. For crying out loud:

> Who on earth would like to program like that? <br/>Even the simplest task results in writing tedious obscure code!

And that's true, if we had to program like that, it would be a nightmare of a job. That's where the distinction between leaf and non-leaf futures becomes very helpful.

You see, this pattern "wait until a lower-level future completes, then progress to the next step" is so mechanical and generic, that we can leave it to the compiler to generate the implementation of all nitty-gritty details of `Future::poll()` and never have to do it manually! Instead, we can focus on what's really important: the algorithm the future implements.

### Compiler support: _async/await_

Let's now see how the compiler helps us escape the nightmare of writing future implementations from the ground up, with the help of `async/await` syntax. Spoiler alert: using `async/await`, we can write the code in a much more direct and readable way. It looks almost like regular synchronous code, with a sprinkle of "async magic" here and there.

Assuming that we have an asynchronous implementation of the `TcpStream` type, the code for the `ReadTextFuture` we introduced above would look like this:

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

Looks quite straightforward, right? This function reads almost like a normal synchronous function. Nonetheless, under the hood this is a state machine that orchestrates futures, similar to what we discussed above. Let's have a closer look.

First, the `async` keyword in `async fn read_text(...)` means that calling this function does not execute its body immediately. Instead, it creates and returns a future. As with other Rust futures, that future is lazy: real work starts only when someone polls it.

You'll also notice a few `.await` keywords here and there. Each `.await` is a _suspension point_ (or _yield point_) inside this future. In other words, these are the points where the generated `poll()` can interrupt its flow and return `Pending` to the caller. When called again, it behaves as if execution resumes from that suspension point. There's no magic, though: that behavior is guaranteed by the compiler carefully generating the state machine for us.

# Async runtimes 

By now, we've talked in depth about what futures are and how to implement them, both directly via the `Future` trait and indirectly via async/await. However, we haven't touched yet on an interesting topic: who actually drives futures to completion? Remember, futures by themselves are **inert**, so there's got to be someone to keep polling them to make progress. This is where the idea of an _async runtime_ enters the scene.

### What a runtime does 

The basic idea behind an async runtime is simple: given a future, keep calling its `poll()` method until the future resolves to a result. A naive implementation comes to mind immediately: just keep calling `poll()` in a loop until it returns `Ready(T)`. It would work, but obviously that's going to be very wasteful, keeping the CPU busy in that loop while the future is pending. A more mature runtime should provide at least these capabilities:

* run multiple futures concurrently;
* schedule futures efficiently, so that ones that are not yet ready to progress don't waste CPU time.

In particular, as our experience with `mio` illustrated, the OS provides us with mechanisms to avoid active polling via I/O event queues. The async runtime should make use of these capabilities to make sure that futures that are currently waiting for an I/O event don't get polled needlessly. On the other hand, once the I/O resource becomes ready, we'd like the runtime to poll that future as soon as possible.

### Choose your runtime 

Interestingly, unlike many other programming languages, Rust doesn't come with a "standard" async runtime. Instead, runtimes are installed as separate crates. This decision, like almost everything in software development, has both pros and cons.

On the positive side, it gives developers a lot of flexibility in choosing an optimal runtime according to their project's needs and constraints. It also allows runtime implementations to evolve more quickly because they are not constrained by the release cycle of Rust's standard library.

On the negative side, it creates a bit of a mess in the async Rust ecosystem. Runtime-specific I/O types and services are generally not interoperable, so mixing runtimes can be awkward or impossible. For application developers, it's less of a problem: usually you just pick a single async runtime for your project and stick to it. For library developers, however, it's a much bigger pain. If you aim to develop a library with async features, you either need to pick a single runtime that you're going to support or go to great lengths to make your library compatible with different runtimes.

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

The reactor and executor work together in a loosely coupled coordination: the executor provides each task with a [_Waker_](https://doc.rust-lang.org/std/task/struct.Waker.html), and when the reactor observes a ready event, it uses that waker to mark the task as runnable, so the executor can poll it.

In practice, resources are usually tightly coupled to the runtime's reactor and wakeup machinery. That coupling is one of the main reasons interoperability between async runtimes in Rust is limited.

# What's next?

So, that was a brief overview of async Rust. Next, we'll dive into the deep end of the pool and implement our own little async runtime. After all, since Rust allows it, why not build one ourselves? Though it may sound like reinventing the wheel, I think it's a good exercise to learn how async runtimes work under the hood. Let's do it!

[last-post]: {{site.baseurl}}/{% post_url 2026-04-10-non-blocking-request-file %}
[last-post-reflections]: {{site.baseurl}}/{% post_url 2026-04-10-non-blocking-request-file %}#lessons-learned-the-hard-way
[last-post-state-machine]: {{site.baseurl}}/{% post_url 2026-04-10-non-blocking-request-file %}#implementing-the-state-machine
