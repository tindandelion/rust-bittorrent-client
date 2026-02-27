---
layout: post
title:  "Non-blocking I/O: Connect to multiple peers in parallel"
date: 2026-02-20 
---

# What errors do we get from peers? 

I was interested to see what kinds of errors we usually get when we try to connect to a remote peer. To dive into it, I wrote a simple [side program][link-missing] to collect that data. That program just iterates over the collection of peer addresses received from the torrent tracker, and tries to request the file from each peer. It counts the types of errors by type, along with the time it took to receive the response. 

Running this program, we get the following statistics at the end: 

```text
 --- Errors by time spent (total time 116947 ms):
0:      connection timed out: 20 (total 100013 ms)
1:      failed to fill whole buffer: 21 (total 4271 ms)
2:      Connection reset by peer (os error 54): 3 (total 449 ms)
3:      IncompleteFile: 1 (total 245 ms)
4:      Connection refused (os error 61): 1 (total 207 ms)
```

That result has in fact confirmed my suspicions. As you can see, we waste most of the time trying to connect to unresponsive peers, only to get the "connection timed out" in the end. This is good news, actually: establishing the TCP connection with a peer is the very first step in the interaction, so we can detect and discard unresponsive peers very early on. If we focus only on that part of interaction and manage to optimize it, we'll already have reduced the time to request the file by a very large amount. 

The general idea for optimization really lies on the surface: instead of trying each peer one by one in sequence, we can try to connect to all of them in parallel, and then continue working with the one that responds first. 

The very first solution that comes to mind is to use multiple threads: just spawn as many worker threads as there are IP addresses, and let each thread try to connect to the remote host. When the worker connects or receives an error, it can report to the main thread via [MPSC channel][mpsc-channels-post]. The main thread simply waits for the first successful message in the channel, and then proceeds with the connected TCP stream. 

In fact, this is a pretty viable solution. However, it doesn't look that appealing to me, for two reasons. 

The first reason is that there's not a lot of new learning opportunities. I already know how to work with threads and channels in Rust, so it won't be a big challenge to implement that solution. I'd like to try something new instead. 

The second reason is that this solution is unnecessarily wasteful. The majority of spawned threads will just be sitting there doing nothing. However, each thread comes with an overhead: the OS allocates a sizable chunk of user space memory to maintain the thread's stack, plus some kernel memory for thread bookkeeping. The actual size of allocated memory depends on the operating system, but it can be as large as 8 megabytes per thread's stack for 64-bit Linux architectures. 

Granted, in our case that memory overhead is not a big problem: we'd only spawn around 50 threads, which is peanuts for the modern hardware. That overhead usually becomes a bottleneck in heavily loaded server applications. However, for sake of learning, let's pretend that we are memory-bound and that overhead is really a problem. 

It's not a strictly theoretical problem, indeed. Let's imagine that I want to run a BitTorrent client on an AWS instance, and I want to use the smallest possible instance type, to save the costs. As of 2026, the smallest instance type is [t4g.nano](https://aws.amazon.com/ec2/instance-types/t4/), which only gives us 512 MB of memory. For sure, in such a tight environment we would like to use as little memory as possible, and wasting 400 MB (50 threads &times; 8 MB stack size) becomes very unreasonable. 

Fortunately, a better solution exist, that allows us to handle multiple TCP connections concurrently using only a single thread. Enter _Non-blocking I/O_. 


# I/O: blocking vs non-blocking

So far, we've been using _blocking I/O_ to communicate with peers over TCP. In that mode, the operating system suspends the execution thread until the I/O operation is complete. For example, when the program makes a `connect()` syscall to establish a TCP connection over a socket, it does not return until the connection is established, or an error condition occurs. Similarly, the call to `read()` to read data will not return until the data is ready. 

In many cases, this is what the programmer wants. For example, suppose we write a simple program that talks to a remote service. There's not much that program can do until it receives a response from the service, so waiting idly until it receives the data is a sensible thing to do. 

The advantage of blocking I/O is its simplicity for the programmer: the program is a linear sequence of I/O operations. 

Things get different when there is work that the program could do while waiting for a response from the remote server. 

# Non-blocking I/O

[mpsc-channels-post]: {{site.baseurl}}/{% post_url 2026-01-12-background-tasks-ratatui %}#inter-thread-event-channel