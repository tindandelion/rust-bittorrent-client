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

That result in fact confirmed my suspicions. As you can see, we waste most of the time trying to connect to unresponsive peers, only to get the "connection timed out" in the end. This is good news, actually: establishing the TCP connection with a peer is the very first step in the interaction, so we can detect and discard unresponsive peers very early on. If we focus only on that part of interaction and manage to optimize it, we'll already have reduced the time to request the file by a very large amount. 

The general idea for optimization really lies on the surface: instead of trying each peer one by one in sequence, we can try to connect to all of them in parallel, and then proceed with the one that responds first. 



