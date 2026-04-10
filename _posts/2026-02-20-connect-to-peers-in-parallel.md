---
layout: post
title:  "Non-blocking I/O: Connect to multiple peers concurrently"
date: 2026-02-20 
---

In this section, I'm taking a first shot at dealing with multiple peers concurrently using _non-blocking I/O_. We'll revisit the process of connecting to remote hosts and try to eliminate the biggest time-sucker we've had so far: **connection timeouts**. 

[*Version 0.1.2 on GitHub*][github-0.1.2]{: .no-github-icon}

# What errors do we get from peers? 

I was curious to see what kinds of errors we usually get when we try to connect to a remote host. To give me some insight, I wrote a small [side program](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/examples/request-file.rs) to collect that data. This program receives the list of peers from the torrent tracker and tries to request the file from each peer. It counts errors by type, along with the time it took to receive the response. 

Running this program, we get the following statistics at the end: 

```text
 --- Errors by time spent (total time 116947 ms):
0:      connection timed out: 20 (total 100013 ms)
1:      failed to fill whole buffer: 21 (total 4271 ms)
2:      Connection reset by peer (os error 54): 3 (total 449 ms)
3:      IncompleteFile: 1 (total 245 ms)
4:      Connection refused (os error 61): 1 (total 207 ms)
```

That result confirmed my intuitions. As you can see, we waste most of the time trying to connect to unresponsive hosts, only to get the "connection timed out" in the end. This is good news, actually: establishing the TCP connection with a peer is the very first step in the interaction, so we can detect and discard unresponsive peers very early on. If we focus only on that part of the interaction and manage to optimize it, we'll already have reduced the time to request the file by a very large amount! 

The general idea for optimization is simple: instead of trying each peer one by one in sequence, we can try to connect to all of them in parallel, and then continue working with the one that responds first. 

# Why not multiple threads? 

The very first solution that comes to mind is to use multiple threads: just spawn as many worker threads as there are IP addresses, and let each thread try to connect to the remote host. When the worker connects or receives an error, it should report to the main thread via [MPSC channel][mpsc-channels-post]. The main thread simply waits for the first successful message in the channel, and then proceeds with the connected TCP stream. 

In fact, this is a pretty viable solution. However, it doesn't look that appealing to me, for two reasons. 

The **first reason** is that there's not a lot of new learning opportunities. I already know how to work with threads and channels in Rust, so it won't be a big challenge to implement that solution. I'd like to try something new instead. 

**Another reason** is that this solution is unnecessarily wasteful. The majority of spawned threads will just be sitting there doing nothing. However, each thread comes with an overhead: the OS allocates a sizable chunk of user space memory to maintain the thread's stack, plus some kernel memory for thread bookkeeping. The actual size of allocated memory depends on the operating system, but it can be as large as 8 megabytes per thread's stack for 64-bit Linux architectures. 

Granted, in our case that memory overhead is not a big problem: we'd only spawn around 50 threads, which is peanuts for the modern hardware. That overhead usually becomes a bottleneck in heavily loaded server applications. However, for the sake of learning, let's pretend that we are memory-bound and that overhead is really a problem. 

It's not a strictly theoretical problem, indeed. Let's imagine that I want to run a BitTorrent client on an AWS instance, and I want to use the smallest possible instance type, to save the costs. As of 2026, the smallest instance type is [t4g.nano](https://aws.amazon.com/ec2/instance-types/t4/), which only gives us 512 MB of memory. For sure, in such a tight environment we would like to use as little memory as possible, and wasting 400 MB (50 threads &times; 8 MB stack size) becomes very unreasonable. 

Fortunately, a better solution exists that allows us to handle multiple TCP connections concurrently using only a single thread. Enter _non-blocking I/O_. 


# I/O: blocking vs non-blocking

So far, we've been using _blocking I/O_ to communicate with peers over TCP. In that mode, any potentially long-lasting I/O system call, such as `connect()` or `read()`, will block until the operation is complete. The OS suspends the calling thread and wakes it up only when the data is ready, or an error has occurred. 

Blocking I/O is a very convenient model for programmers in many cases. First, the program flow is easy to follow: it consists of subsequent calls to I/O system calls. Second, in many cases the program has nothing to do but wait until the data is ready: the OS does its best to keep the thread suspended while the I/O operation is still in progress. 

Things are different, though, when there is other work that the program can do while waiting for an I/O operation to finish. In that case, blocking calls become a limitation rather than assistance. Hence, I/O operations support so-called _non-blocking I/O_. 

In non-blocking mode, I/O system calls (for example `connect()` or `read()`) return immediately. If the operation is not ready yet, the OS signals this with an error code `EWOULDBLOCK` or `EAGAIN` (depending on the operating system). In that case, the caller can switch to doing some other work, and retry the operation again according to its own logic. 

#### OS event queues 

So non-blocking I/O gives programmers much more flexibility about what to do while the I/O operation is still ongoing. But then the question becomes: what should we do when we've run out of all available work and really need to wait for the I/O operation to finish? 

One possible approach could be polling. Speaking abstractly, we could query the OS in a loop to see whether the I/O operation has completed. The actual mechanism by which we can obtain the status of the operation depends on its type, but generally we can get the operation status one way or another. 

Polling isn't very efficient, though. If we poll too frequently, we wake up the thread unnecessarily, thus occupying CPU time just to ask for the updated status. If we poll at longer intervals, we end up reacting too slowly. We need a solution that could allow us to avoid unnecessary thread wake-ups, but also to react to the I/O events quickly without long delays. 

Luckily, modern operating systems provide us with such a mechanism: event queues. OS event queues work in conjunction with the non-blocking I/O as follows: 

* The application creates an event queue and registers event sources, such as sockets or file descriptors, in that queue, along with some additional information: event types, trigger conditions, etc. One queue can handle multiple event sources, acting as a multiplexer. 

* When the application needs more data to move forward, it polls the queue. The OS suspends the calling thread and wakes it up only when there's an event waiting to be processed; 

* The application wakes up, receives the event and moves forward. 

Thus, event queues give us a tool to facilitate the following tasks: 

* Wait for I/O events efficiently, avoiding unnecessary thread wake-ups when the data is not ready yet; 
* Merge events from different sources, allowing us to use a single event loop to process multiple I/O sources concurrently. 

#### Dealing with the zoo: _mio_ library

Unfortunately, there's no consensus among different platforms in event queue implementations: 

* **epoll** API in Linux systems; 
* **kqueue** API in BSD systems, including MacOS; 
* **IOCP** in Windows-based systems

All these APIs essentially serve the same purpose of managing I/O events, but differ in behaviour and implementation details, which complicates cross-platform application development. It's no wonder that over time a bunch of cross-platform libraries have emerged, that provide a layer of abstraction on top of OS-specific APIs and mitigate the platform-specific differences. 

In Rust, the most popular library for working with non-blocking I/O is [_mio (Metal I/O)_](https://docs.rs/mio/latest/mio/): 

> Mio is a fast, low-level I/O library for Rust focusing on non-blocking APIs and event notification for building high performance I/O apps with as little overhead as possible over the OS abstractions.

As of version _1.1.1_, **mio** provides support for a variety of platforms: 

* Linux; 
* Windows;
* BSD-based: macOS, OpenBSD, FreeBSD, etc.; 
* Mobile: iOS, Android.

In a nutshell, mio is an obvious choice for programming non-blocking I/O in Rust, so let's give it a shot. 

# Connecting to peers concurrently

The first thing I decided to do was to hide the pesky details of connecting to peers (concurrent or otherwise), and provide a nice abstraction that I could use in the main app code. In my mind, I'd like to have an entity with the following simple API: 

```rust
struct PeerConnector { ... }

impl PeerConnector { 
    pub fn connect(self, peer_addrs: Vec<SocketAddr>) -> impl Iterator<Item = TcpStream> { ... }
}
```

Essentially, we could give `PeerConnector` a list of socket addresses, and it would return an `Iterator` over connected `TcpStream`s of peers. The clients of this struct are not interested in how exactly the connection process goes: the details are hidden behind that simple API. 

#### Old logic: _SeqPeerConnector_

So I ended up with two different implementations that support that kind of API. The first one is [`SeqPeerConnector`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/seq_connector.rs) that uses our familiar blocking I/O. This is more of a refactoring artifact: it emerged as I was refactoring the existing code into a new code structure. I think this implementation will go away in the next version, but I keep it for now for nostalgic reasons. 

#### New logic: _ParPeerConnector_

The second implementation is [`ParPeerConnector`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/par_connector.rs), and that's where our non-blocking I/O lives. That struct itself is simply an entry point; the bulk of the logic is spread across a couple of helper data types: `PeerProbe` and `PeerPoller`. 

[`PeerProbe`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/par_connector.rs#L167) is a struct that tracks the current [state](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/par_connector.rs#L161) of connection process for each individual IP address. It starts in `Connecting` state and then updates the state when its [`handle_connect_event()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/par_connector.rs#L197) method is called: 

* We check if the underlying `TcpStream` is connected by querying its `peer_addr()` method;
* If `peer_addr` returns `Ok` we consider that stream connected and enter the `Connected` state;
* If `peer_addr` results in `Err`, we assume that the peer refused to connect and enter the `Error` state. 

The polling logic itself resides in the [`PeerPoller`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.2/src/downloader/peer_connectors/par_connector.rs#L60) helper struct that implements `Iterator` over `TcpStream`s. `PeerPoller` keeps a list of active probes, one per IP address, and manages that list at every call to `Iterator::next()` in a loop: 

1. We first search for a probe that's already in the `Connected` state. If such a probe is found, we remove it from the list of active probes and return its `TcpStream` as the result of `Iterator::next()`; 
2. If there are no connected probes at the moment, we poll the event queue by calling mio's [`Poll::poll()`](https://docs.rs/mio/latest/mio/struct.Poll.html#method.poll) method. That puts our thread to sleep until there are events ready to be handled; 
3. When the `poll()` returns, we receive the list of I/O events that have occurred, and update the state of affected probes. 
4. Finally, we remove errored probes from the active probe list and repeat from step _1_. 

On occasion, `poll()` would finish because of the timeout, and the event list ends up empty. In that case, we assume that all remaining peers were unresponsive, and we return `None` as a result of `next()`. To the caller it signals that there are no more peers to work with. 

# Does it work? 

So let's recap what our `ParPeerConnector` does: 

* When we call `ParPeerConnector::connect()` it sends a non-blocking connect request to all peers from the list and returns an iterator over `TcpStream`s; 
* Each call to `next()` either returns immediately, or blocks until we connect to _any_ of the not-yet-connected peers;
* Once we've obtained a connected `TcpStream`, we can proceed to request a file and start downloading; 
* If the peer refuses to serve the torrent, we call `next()` again to get the next connected `TcpStream`. 

It's worth emphasizing that we only handle `connect` in a non-blocking manner. **The rest of the communication still happens in the old-fashioned blocking way**. However, that's a good starting point that addresses the [biggest annoyance](#what-errors-do-we-get-from-peers) we've had so far:

![Application UI]({{ site.baseurl }}/assets/images/connect-to-peers-in-parallel/main.gif)

Great! Looks like a significant improvement compared to the [previous behavior][prev-screenshot]!

[*Current version (0.1.2) on GitHub*][github-0.1.2]{: .no-github-icon}

[mpsc-channels-post]: {{site.baseurl}}/{% post_url 2026-01-12-background-tasks-ratatui %}#inter-thread-event-channel
[github-0.1.2]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.1.2
[prev-screenshot]: {{site.baseurl}}/{% post_url 2026-02-01-connecting-ui %}#wiring-the-whole-app-together