---
layout: post
title:  "More non-blocking I/O: request file in non-blocking way"
date: 2026-03-24 
---

TBD

A while back, [I described][link-missing] the initial message exchange peers need to complete before we can start receiving file chunks. The short version: both sides send a few setup messages, and eventually the remote peer sends `unchoke` to say, "I’m ready for your requests."

![Request file message exchange]({{ site.baseurl }}/assets/images/more-non-blocking-io/request-file-sequence.svg)

In practice, this setup can take a while. For example, in the [last post][link], we saw that many peers never respond to the initial connection attempt and just time out. That was one of our biggest delays, and we improved it by connecting to all peers concurrently with non-blocking I/O.

But that is not the only bottleneck. Another big delay comes from waiting for `unchoke`. In my tests, peers usually send `handshake` and `bitfield` quickly once the TCP connection is up, but `unchoke` can take much longer, sometimes tens of seconds. And the first peer to connect is not always the first one to unchoke us. So we can easily end up waiting on a slow peer while a faster one is sitting right there.

Here is a simple timeline that shows how this can happen:

![Connect to two peers timeline]({{ site.baseurl }}/assets/images/more-non-blocking-io/connect-timeline.svg)

In this scenario, peer 1 connects first, so we pick it for downloading. But it is slow to send `unchoke`, and we burn almost 10 seconds waiting. Peer 2 connects a bit later, but it is less busy and could send `unchoke` just one second after connecting. If we had picked peer 2, we could have started downloading after about 2 seconds instead of 12.

The tricky part is that we cannot know ahead of time how long a peer will take to unchoke us. In the worst case, `unchoke` never arrives. The best way to avoid this is to keep talking to multiple peers in parallel until one of them finally sends `unchoke`.

# Probe state machine 

In the last post, I introduced the idea of a _peer probe_: a simple state machine driven by I/O events from the underlying TCP stream:

![Probe state diagram]({{ site.baseurl }}/assets/images/more-non-blocking-io/state-diagram-1.svg)

The state machine is intentionally simple: once we get an I/O event from the TCP channel, we move to either `Connected` or `Error`, depending on the socket state.

Now let’s extend it a bit. After the TCP connection is established, we send a handshake and move into a new `Handshaking` state. From there, we keep waiting for I/O events until data arrives from the peer. Once it does, we parse that data as the handshake response and transition to `Connected`.












