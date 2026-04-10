---
layout: post
title:  "Reflection: the need for more non-blocking I/O"
date: 2026-03-24 
---

Let's take a step back and reflect on the progress we've made so far. We've been working on this project for a long time, with multiple side quests, so it's good to check where we stand and what to focus on next.

# Where do we stand? 

A while back, [we discovered][downloading-file-block] that to start receiving data from a remote peer, we first need to perform an initial message exchange, as defined by the BitTorrent protocol specification. Eventually, the remote peer sends us the `unchoke` message to say, _"I'm ready for your requests"_:

![Request file message exchange]({{ site.baseurl }}/assets/images/more-non-blocking-io/request-file-sequence.svg)

For a long time, our default approach was to enumerate peers one by one and perform the message exchange above to find one to work with. That proved to be [a working solution][downloading-whole-piece], but it was rather inefficient: of all peer addresses returned by the torrent tracker, only a small fraction were responsive at all or willing to serve us data. So when we work with peers sequentially, we waste a lot of time trying to talk to unresponsive peers until we finally find one that works.

As I observed the behavior of remote peers, I noticed the following patterns:

* More than half of peers are unreachable, which causes TCP connection attempts to time out. That was one major source of delay for us, since the only way to detect this situation was to wait.
* Once connected, remote peers usually respond to handshake messages quickly. Many simply close the connection after receiving a handshake, while others send a handshake response without major delays.
* Peers are much slower to send the final `unchoke` message: it may take tens of seconds before we get unchoked. In extreme cases, a remote peer may *never* send `unchoke` at all.

The solution [I introduced in the last post][connect-to-peers-in-parallel] was to work with multiple peers at the same time using non-blocking I/O and I/O event queues. In that post, I focused only on the first part of the problem, connection timeouts, to test how this idea works in practice. It proved successful and significantly reduced connection time.

However, we only addressed the first part of the problem. The second major source of delay (waiting for the `unchoke` message) is still there. To illustrate the problem, let's consider the following timeline:

![Connect to two peers timeline]({{ site.baseurl }}/assets/images/more-non-blocking-io/connect-timeline.svg)

In this scenario, peer 1 connects first, so we pick it as our working peer. But it turns out to be slow to send `unchoke`, and we waste almost 10 seconds waiting. At the same time, peer 2 connects a bit later, but it is less busy and could send `unchoke` much faster, just 4 seconds after connecting. If we had picked that peer, we could have started downloading in 6 seconds overall.

Unfortunately, there's no way to know ahead of time which peer will unchoke us first. The only way to optimize waiting time is to continue talking to all connected peers in parallel until one of them finally sends `unchoke`. Only then can we drop the others and focus on downloading file contents from that peer.

The good part is that we already know the mechanism for working with multiple peers concurrently: non-blocking I/O. The bad part is that it will be trickier to implement than the straightforward blocking I/O approach.

Still, let's buckle up and do it! Next, we’ll model the entire message exchange as a state machine that keeps multiple peers in play until one unchokes.

[downloading-file-block]: {{site.baseurl}}/{% post_url 2025-07-17-downloading-file-block %}
[downloading-whole-piece]: {{site.baseurl}}/{% post_url 2025-07-19-download-the-whole-piece %}
[connect-to-peers-in-parallel]: {{site.baseurl}}/{% post_url 2026-02-20-connect-to-peers-in-parallel %}