---
layout: post
title:  "Non-blocking I/O: Request the file from a peer"
date: 2026-04-10 
---

TBD: Description


# From a linear algorithm to a state machine

When working with non-blocking I/O, we need to change the way we think about our program flow. In the blocking mode, the program would look like a linear sequence of instructions. That's no longer true in non-blocking mode. Conversely, we need to approach the algorithm that we implement as a _state machine:_ 

* the machine's states represent the points where we wait for data to be available from the underlying resource; 
* the bits of business logic are split across different state transitions; 
* the entire execution is driven by I/O events coming from the resource via the event queue. 

It sounds rather involved, but transforming a sequential algorithm into a state machine is not that hard, at least in concept. The annoying part is the implementation, though: it requires quite a bit of repetitive code, and the program structure becomes more obscure. There are also a few pesky low-level details that need to be taken care of. 

Because I'm not looking for easy ways in this project, let's go down that route and implement the entire initial message exchange in a non-blocking manner, so that we can communicate with multiple peers concurrently. 

# Modeling the state machine

Let's one more time take a look at the message exchange we need to do with the peer before we can start downloading file data: 

![Request file message exchange]({{ site.baseurl }}/assets/images/non-blocking-request-file/request-file-sequence.svg)

There's a few waiting points here, all related to receiving the data from the remote peer: 

* TCP connection to be established; 
* the handshake message from the remote peer; 
* the `bitfield` and `unchoke` messages. 

To be precise, _sending messages_ is also a non-blocking operation. If the application tries to send vast amounts of data over TCP channel, it could receive `EWOULDBLOCK` on a write operation as well, when the TCP channel can't keep up. However, in our case we only send tiny bits of data, so for sake of simplifying the picture, I would like to send operations as if they were blocking. It's the receiving part that's our primary focus. 

With all that said, here's the state diagram that represents the entire message interchange, from connecting to receiving the `unchoke` message: 

![Probe state diagram]({{ site.baseurl }}/assets/images/non-blocking-request-file/probe-state-diagram.svg)

Now it's time to covert this conceptual picture into code.

# Implementing the state machine

In fact, we have implemented the majority of the state machine operation already, back when we [started to handle TCP connect][connect-to-peers-in-parallel] in non-blocking mode. This code mostly remains unchanged, with a few refinements. The bulk of the changes goes into `ProbeState` enum, where we will put the state transition logic. 

The overall structure of the implementation looks like this: 

![PeerConnector structure]({{ site.baseurl }}/assets/images/non-blocking-request-file/peer-connector-structure.svg)

As before, [`PeerPoller`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector.rs#L71) is responsible for managing the event queue. When the I/O event arrives, it picks the appropriate probe from the list of all probes, and calls its [`PeerProbe::handle_event()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector.rs#L218) method. This method does some necessary housekeeping, such as handling error situations, but its main responsibility is to drive the state machine forward with the following logic: 

* It calls [`ProbeState::update()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector/probe_state.rs#L64) that either succeeds with the new state, or returns an error; 
* on success, we record the returned value as a new state of the probe; 
* if the call to `update()` fails with `ErrorKind::WouldBlock` that means we don't have enough data to move to the next state, so we stay in the current state until the new I/O event occurs; 
* all other error results are considered unexpected errors, and the probe moves to the `ProbeState::Error` state.

Finally, [`ProbeState`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector/probe_state.rs#L29) is an enum that represents the current state of the state machine. There are different ways of implementing states of a state machine in code, but I'd say an enum is probably a natural first choice. Its main method [`ProbeState::update()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector/probe_state.rs#L64) is where we put all the logic of communicating with the peer and corresponding state transitions. 

State transition logic in general follows the following pattern: 

* Receive the data from the TCP channel; 
* Check that data is correct and sensible; 
* Send the next message in the sequence to the TCP channel; 
* Return the new state. 

And that's probably it! Right? 

# Well, it's not that simple

Unfortunately, the reality turned out to be more complicated. When I finished with the implementation of the state machine above, and looked closely at its behaviour, strange things started popping up: 

* The [unit tests](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector.rs#L274) for `PeerConnector` became unstable: roughly, they failed in 10% of runs for some obscure reason; 
* I started to see the errors from the TCP layer that I hadn't seen before, and I had no explanation for them. 

It took me a few hours of debugging to get down to the root of the issue. You see, in fact we have not one, but two layers of events. On a higher level, we have the events that drive our state machine: basically, events of the types _"The message from the remote peer has arrived"_. That's the sort of events we talk about when we describe the behavior of the state machine. 

On the low level of TCP sockets, we have I/O events, that basically say _"Some data is available to read from the TCP socket"_. The tricky part is that these two types of events don't map one-to-one. In particular, there's 2 distinct situations we need to tackle: 

* A single I/O event was fired when several messages have arrived; 
* An I/O event was fired when only a part of the message has arrived. 

Let's see when these situations occur and how to deal with them. 

#### One I/O event for several BitTorrent messages

I observed this situation only in the local tests, because it is predicated on the condition that the data from the remote peer arrives very quickly. However, there's no guarantee that it can't happen "in production". The root cause was that the remote peer was sending us the response handshake and the `bitfield` so quickly, that they only generated one I/O event: 

![Receiving several messages with a single I/O event]({{ site.baseurl }}/assets/images/non-blocking-request-file/many-messages-single-event.svg)

What happens in this scenario if we only advance our state machine by I/O events? Well, that event makes the state machine transition from `Handshaking` to `WaitingForBitfiield` state as expected, but then it gets stuck there. Even though the bitfield data is already available in TCP socket, we never receive a separate event for it, so the state machine never gets to process that data! It's stuck in the `WaitingForBitfield` state forever. 

The remedy for this situation is that we need to be proactive when reading data from the TCP socket: as soon as the I/O event has arrived, we should try to advance the state machine as far as possible, unless the read operation results in `EWOULDBLOCK` error. Only when we receive that error code, should we stop and wait for the next I/O event to occur. 

At the extreme case, our code should be able to handle the situation when the remote peer unchokes us instantly and _all_ messages are received at once: 

![Receiving all messages with a single I/O event]({{ site.baseurl }}/assets/images/non-blocking-request-file/all-messages-single-event.svg)

Here, the state machine should go from `Handshaking` to `Unchoked` instantly, passing through all intermediate states without waiting.

#### A BitTorrent message split across several I/O events 

The second scenario we have to account for is the opposite. Sometimes, a single BitTorrent message can be split across several I/O events. In real life, I observed this situation occur quite regularly when we were expecting the `bitfield` message from the peer:  

![Single message spread across multiple I/O events]({{ site.baseurl }}/assets/images/non-blocking-request-file/single-message-many-events.svg)

What it means for us is that we have to be ready that, when the I/O even occurs, only a part of the BitTorrent message is available to read. In that case, our code has to read the available data, store it in the intermediate buffer, and then wait for the next I/O event to read the next portion. It should keep doing that until the entire BitTorrent message has been received. 

In particular, that means we can't rely on [`Read::read_exact()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact) method when we work with TCP streams in non-blocking mode. Its implementation does not handle `EWOULDBLOCK` errors nicely: it just returns the error and loses all partial data it has read from the TCP stream. 

Fortunately, the [BitTorrent message format][bittorrent-message-format] allows us to write a custom reading routine that can read the message contents in chunks, accumulating the partially read data in the intermediate buffer: 

* At the start, we expect to receive 4 bytes of data that contains the total message length; 
* Once we have these 4 bytes, we know the total length of the incoming message, so we can keep accumulating the partial data in the buffer until the entire message is received; 
* When we've accumulated the needed number of bytes, we can construct the `PeerMessage` value and return it to the caller. 

That routine is now implemented in a [`MessageBuffer`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.3/src/downloader/peer_connector/message_buffer.rs) helper type.

# Trying it out 

Okay, there's been quite a lot of changes in the code. Let's now try to run our main program and see the effects with our own eyes: 

![Application UI]({{ site.baseurl }}/assets/images/non-blocking-request-file/main.gif)

Amazing! First of all, it works. Second, even with a naked eye we can see the improvement, when comparing with [the previous iteration results][prev-iteration]: now the file download starts almost instantly. Our efforts of going through the hurdles of programming non-blocking I/O have paid off! 

# Lessons learned the hard way

[When I just started][non-blocking-io-reasoning] to explore working with non-blocking I/O, I contemplated a simpler solution that would use convenient blocking I/O and multiple threads, to achieve the same goal. Back then, I decided to embark on a non-blocking journey, mostly out of curiosity. Now, once we have a working solution with non-blocking I/O, the time has come to reflect on that decision, and ask the question: **was it worth it?**

Well, to be completely honest, if it were a real-life project with real consequences, my answer would be resounding: 

> No, using non-blocking I/O is not justified in this particular project. 

It's true that non-blocking I/O gives us a useful tool to handle multiple TCP sockets at the same time with the minimal resource overhead. However, it comes with a significant cost from the developer effort: 

* The code to handle socket communication becomes very hairy very fast. Instead of a simple linear sequence of instructions we have to deal with a state machine implementation, which is much more obscure and hard to understand for the developer. Even though the process of transforming the linear algorithm into a state machine is pretty straightforward on paper, the code we had to produce is quite hard to reason about, especially if we had to introduce it to a new developer, or even for ourselves when we come back to this code a few months later; 

* Non-blocking I/O brings about a number of special cases that need to be handled, and it's much more prone to developer errors. The subtle bugs I reasoned about in the [previous section][prev-section] were quite subtle, and frankly, it was a bit of luck that I was able to notice them at all! I wonder how many bugs are still there, those that I haven't yet noticed. 

To summarize, it was a very interesting experience from the perspective of learning to work with non-blocking I/O. However, for a real-world projects, I'd recommend to stick to a much simpler solution with multiple threads. Only if we have 100% confidence that multiple threads cause serious performance problems, should we contemplate switching to non-blocking I/O and suffer the increased costs of development and maintenance efforts. 

# Next steps 

TBD

[connect-to-peers-in-parallel]: {{site.baseurl}}/{% post_url 2026-02-20-connect-to-peers-in-parallel %}
[bittorrent-message-format]: {{site.baseurl}}/{% post_url 2025-07-17-downloading-file-block %}#peer-message-format
[prev-iteration]: {{site.baseurl}}/{% post_url 2026-02-20-connect-to-peers-in-parallel %}#does-it-work
[non-blocking-io-reasoning]: {{site.baseurl}}/{% post_url 2026-02-20-connect-to-peers-in-parallel %}#why-not-multiple-threads