---
layout: post
title:  "Selecting the peer with the complete file"
date: 2025-08-05
---

In our current implementation, we communicate with only one peer to download the file. It imposes some additional restrictions upon which peer we're going to select. In this section, I discuss these restrictions and their implementation in code. 

[*Version 0.0.10 on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.10){: .no-github-icon}

# How to choose the peer to download from 

Let's think about which peer we should pick for downloading the file. First and foremost, obviously, we should be able to connect to that peer and exchange the handshake messages. That's the bare minimum: passing this step will ensure us that there's a BitTorrent client on the other side. But that's not all we should require from the remote peer. 

You see, our current implementation downloads the file from a single peer, so the other requirement is that the peer we must have the entire file available. In general, that's not necessarily the case: in BitTorrent network the peers may have only parts of the file. Trying to download the file from such a peer would be a mistake in our current implementation: our client will just hang forever, waiting for missing file pieces. To avoid this misfortune, we should make sure that the peer we select to communicate with has the entire file. 

Luckily, we can easily check the completeness of the file on the other end. Remember that right after the handshake, the remote peer sends us the `bitfield` message that contains the information about which pieces are available for download, in a form of a bitfield. All we need to do is to check that all pieces are present. That's going to be the second selection criterion for us. 

Next, to initiate the download, our client sends the `interested` message and expects to receive `unchoke` message in return. But that may never happen: the remote peer may never "unchoke" us, for reasons unknown. Currently, our client will just quit with the timeout error. However, there's a smarter strategy to handle this situation: instead of quitting, we can move on to the next peer in the list and request the download from it. Let's consider it a third selection criterion: the peer should send us `unchoke` message within a reasonable timeout. 

To summarize all the above, here's what we need to do when choosing the remote peer: 

1. Connect to the peer and exchange the handshake messages; 
2. Receive the `bitfield` message and make sure that the peer has the entire file; 
3. Send `interested` message and wait until it responds with `unchoke` message, or the timeout occurs. 

If any of these steps fail, we should move on to the next peer in the list. With some luck, eventually we'll come across the peer that satisfies all criteria. Otherwise, we can quit with a message that we were unlucky this time. 

# Extract the message exchange logic 

Until now, we had the initial message exchange implemented directly in [`main::download_file()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/bin/main.rs#L101) routine in a somewhat ad-hoc manner. It's time to give it a bit more attention and implement it properly. 

I extracted that logic into a separate function [`request_complete_file`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.10/src/downloader/request_download.rs#L21). This function implements steps #2 and #3 from above: 

* The sequence of messages hasn't changed: we still receive `bitfield` message, send `interested`, and wait for `unchoke` message from the other end; 
* The new bit of functionality is that we now utilize the payload of the `bitfield` message: we analyze the contents of the bitfield and make sure that the remote peer contains the entire file. Otherwise, we return `IncompleteFile` error. 

Another notable change is the introduction of [`MessageChannel`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.10/src/downloader/peer_comm.rs#L24) trait to send and receive BitTorrent messages. This is an abstraction that makes `request_complete_file` testable. In production code, we want to send messages over the TCP channel to the remote host. In tests, however, we need to have a "fake" implementation of `MessageChannel` to allows us to simulate different scenarios that may occur in real life: incomplete files, unexpected bitfield data, etc. 

# Summary

With the new functionality in [`request_complete_file`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.10/src/downloader/request_download.rs#L21) we are now more selective about what peer to communicate with: the peer should have the entire file on their end, and it should unchoke us within a reasonable timeout. We still probe peers one by one sequentially, until we've found the peer that satisfies those criteria. 