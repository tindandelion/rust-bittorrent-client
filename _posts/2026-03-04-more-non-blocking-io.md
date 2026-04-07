---
layout: post
title:  "More non-blocking I/O: request file in non-blocking way"
date: 2026-03-24 
---

TBD

A while back [I described][link-missing] I described that in order to start receiving the chunks of the file, the peers need engage in a certain initial message exchange. In a nutshell, there is some back and forth between the peers, and eventually the remote peer sends us the `unchoke` message that signals that it's ready to accept requests for file content: 

![Request file message exchange]({{ site.baseurl }}/assets/images/more-non-blocking-io/request-file-sequence.svg)

We've also discovered in practice that this message exchange can take a significant amount of time. For example, in the [last post][link] we discovered that quite a bunch of peers simply don't respond to the initial connection attempts, and it times out. It was one of the big time-wasters, which we eliminated by connecting to all peers concurrently using non-blocking I/O. 

But that's not the only one bottleneck we can eliminate, though. It turns out that the second major delay comes from waiting for the `unchoke` message from the remote peer. Indeed, what I observed in practice is that once the TCP connection is established, the peers are quite fast to send their handshake and `bitfield` messages, but it can take quite a bit of time (sometimes tens of seconds) to send us back the `unchoke` message. Moreover, it's absolutely not a guarantee that the peer that was the fastest to connect, would also be the fastest to to unchoke us. In practice, this can lead to unfortunate situations when we start communicating with a slow peer, whereas there can be a much faster one. 

Let's see how this may happen from the following hypothetical timeline: 

![Connect to two peers timeline]({{ site.baseurl }}/assets/images/more-non-blocking-io/connect-timeline.svg)

In this scenario, peer 1 was the fastest to establish the connection, so we picked it to download the file. However, it was also quiet slow to send the `unchoke` message, so we wasted almost 10 seconds waiting. At the same time, there was a peer 2 that was a bit slower to connect to, but it also was less busy and could have sent us the `unchoke` message only after 1 second. Had we communicated with this peer instead, we'd start downloading the file only after 2 seconds, instead of 12 seconds with the peer 1. 

Unfortunately, we can't know in advance how long it's going to take the remote peer to unchoke us. In the extreme case, it can happen that we'll never receive the `unchoke` message at all. The only way for us to avoid these suboptimal scenarios is to keep working with multiple peers concurrently until we finally receive `unchoke` message from one of them. 

# Probe state machine 

In the last post, I introduced the idea of the _peer probe_ that essentially implemented a simple state machine that was driven by I/O events from the underlying TCP stream: 

![Probe state diagram]({{ site.baseurl }}/assets/images/more-non-blocking-io/state-diagram-1.svg)

That state machine is very simple: once we receive I/O event from the TCP channel, we switch to the `Connected` or `Error` state, depending on the state of the TCP channel. Now, let's imagine that we don't stop there. Let's suppose that, once connected, we would send a handshake message to the remote peer and enter the new state, `Handshaking`. In that state, we continue waiting for I/O events until we receive the event that signals that there is some data received from the remote host. Once we have it, we can process that data as a response handshake, and enter the `Connected` state. 












