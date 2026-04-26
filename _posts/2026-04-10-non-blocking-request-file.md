---
layout: post
title:  "Non-blocking I/O: request the file from a peer"
date: 2026-04-10 
---

TBD: Description

# Lessons learned the hard way

TBD

# From a linear algorithm to a state machine

When working with non-blocking I/O, we need to change the way we think about our program flow. In blocking mode, the program would look like a linear sequence of instructions. That's no longer true in non-blocking mode. Conversely, we need to consider the algorithm that we implement as a _state machine_: 

* the states in that machine represent the points where we wait for data to be available from the underlying resource; 
* the bits of business logic are split across different state transitions; 
* the entire execution is driven by I/O events coming from the resource via the event queue. 

It sounds rather involved, but transforming a sequential algorithm into a state machine is not that hard, at least in concept. The annoying part is the implementation, though: it requires quite a bit of repetitive code, and the program structure becomes more obscure. There are also a few pesky low-level details that need to be taken care of. 

Because I'm not looking for easy ways in this project, let's go down that route and implement the entire initial message exchange in a non-blocking manner, so that we can communicate with multiple peers concurrently. 

# Modeling the state machine

Let's one more time take a look at the message exchange we need to do with the peer before we start downloading: 

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

In fact, we have implemented the majority of the state machine operation already, back when we [started to handle TCP connect][link?] in non-blocking mode. This code mostly remains unchanged, with a few refinements. The bulk of the changes goes into `ProbeState` enum, where we will put the state transition logic. 

The overall structure of our implementation looks like this: 

[Picture]

As before, [`PeerPoller`][link?] is responsible for managing the event queue. When the I/O event arrives, it picks the appropriate probe from the list of all probes, and calls its [`PeerProbe::handle_event()`][link?] method. This method does some necessary housekeeping, such as handling error situations, but its main responsibility is to drive the state machine forward with the following logic: 

* It calls [`ProbeState::update()`][link?] that either succeeds with the new state, or returns an error; 
* on success, we record the returned value as a new state of the probe; 
* if it fails with `ErrorKind::WouldBlock` that means we don't have enough data to move to the next state, so we stay at the current state until the new I/O event arrives; 
* all other error results are considered unexpected errors, and the probe moves to the `ProbeState::Error` state.

Finally, [`ProbeState`][link?] is an enum that represents the current state of the state machine. There are different ways of implementing states of a state machine in code, but I'd say an enum is probably a natural first choice. Its main method [ProbeState::update()][link?] is where we put all the logic of communicating with the peer and corresponding state transitions. State transition logic in general follows the following pattern: 

* Receive the data from the TCP channel; 
* Check that data is correct and sensible; 
* Send the next message in the sequence to the TCP channel; 
* Return the new state. 

And that's probably it! Right? 

# Well, it's not that simple

Unfortunately, the reality turned out to be more complicated. When I finished with the implementation of the state machine above, and looked at how it behaved, strange things started popping up: 

* The [unit tests][link?] for `PeerConnector` became unstable: roughly, they failed in 10% of runs for some obscure reason; 
* I started to see the errors from the TCP layer that I hadn't seen before, and I had no explanation for them. 

It took me a few hours of debugging to get down to the root of the issue. 

You see, in fact we have not one, but two layers of events. On a higher level, we have the events that drive our state machine: basically, events of the types _"The message from the remote peer has arrived"_. That's the sort of events we talk about when we describe the behavior of the state machine. 

On the low level of TCP sockets, we have I/O events, that basically say _"Some data is available to read from the TCP socket"_. The tricky part is that these two types of events don't map one-to-one. In particular, there's 2 distinct situations we need to tackle: 

* An I/O event was fired when multiple messages have arrived; 
* An I/O event was fired when only a part of the message has arrived. 

Let's see when these situations occur and how to deal with them. 

#### One I/O event for several BitTorrent messages

I observed that situation only in the local tests, because it is predicated on the condition that data from the remote peer arrives very quickly. However, there's no guarantee that it can't happen "in production". The root cause was that the remote peer was sending us the response handshake and the `bitfield` so quickly, that they only generated one I/O event: 

![Receiving several messages with a single I/O event]({{ site.baseurl }}/assets/images/non-blocking-request-file/many-messages-single-event.svg)

What happens in this scenario if we only advance our state machine by I/O events? Well, that event makes the state machine transition from `Handshaking` to `WaitingForBitfiield` state as expected, but then it gets stuck there. Even though the bitfield data is already available in TCP socket, we never receive a separate event for it, so the state machine never gets to process that data! It's stuck in the `WaitingForBitfield` state forever. 

The remedy for this situation is that we need to be proactive when reading data from the TCP socket: as soon as the I/O event has arrived, we should try to advance the state machine as far as possible, unless the read operation results in `EWOULDBLOCK` error. Only when we receive that error code, should we stop and wait for the next I/O event to occur. 

At the extreme case, our code should be able to handle the situation when the remote peer unchokes us instantly and _all_ messages are received at once: 

![Receiving all messages with a single I/O event]({{ site.baseurl }}/assets/images/non-blocking-request-file/all-messages-single-event.svg)

Here, the state machine should go from `Handshaking` to `Unchoked` instantly, passing through all intermediate states without waiting.

#### A BitTorrent message split across several I/O events 

The second scenario we have to account for is the opposite. Sometimes, a single BitTorrent message can be split across several I/O events. In real life, I observed this situation occur quite regularly when we were receiving the `bitfield` message from the peer:  

![Single message spread across multiple I/O events]({{ site.baseurl }}/assets/images/non-blocking-request-file/single-message-many-events.svg)


What it means for us is that we have to be ready that when the I/O even occurs, only a part of the BitTorrent message will be available to read. In that case, our code has to read the available data, store it in the intermediate buffer, and then wait for the next I/O event to read the next portion, and do so until the entire BitTorrent message has been received. 

In particular, that means we can't rely on [`Read::read_exact()`][link?] method when we work with TCP streams in non-blocking mode. Its implementation does not handle `EWOULDBLOCK` errors nicely: it just returns the error and loses all partial data it has read from the TCP stream. 

Fortunately, the [BitTorrent message format][link?] allows us to write a custom reading routine that can read the message contents in chunks, accumulating the partially read data in the intermediate buffer: 

* At the start, we expect to receive 4 bytes of data that contains the total message length; 
* Once we have these 4 bytes, we know the total length of the incoming message, so we can keep accumulating the partial data in the buffer until the entire message is received; 
* When we've accumulated the needed number of bytes, we can construct the `PeerMessage` value and return it to the caller. 

I have implemented that routine in a [`MessageBuffer`][link?] struct. 

