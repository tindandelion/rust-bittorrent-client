---
layout: post
title:  "Non-blocking I/O: request the file from a peer"
date: 2026-04-10 
---

When working with non-blocking I/O, we need to change the way we think about our program flow. In blocking mode, the program would look like a linear sequence of instructions. That's no longer true in non-blocking mode. Conversely, we need to consider the algorithm that we implement as a _state machine_: 

* the states in that machine represent the points where we wait for data to be available from the underlying resource; 
* the bits of business logic are split across different state transitions; 
* the entire execution is driven by I/O events coming from the resource via the event queue. 

It sounds rather involved, but transforming a sequential algorithm into a state machine is not that hard, conceptually. The annoying part is the implementation, though: it requires quite a bit of repetitive code, and the program structure becomes more obscure. There are also a few pesky low-level details that need to be taken care of. 

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

The structure of our implementation looks like this: 

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

And that's probably it! 

# Well, it's not that simple



