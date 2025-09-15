---
layout: post
title:  "Refactor probing peers"
date: 2025-08-05
---

TODO: Small summary 

# Choose the peer to download from 

Let's think about which peer we should pick for downloading the file. First and foremost, obviously, we should be able to connect to that peer and exchange the handshake messages. That's the bare minimum: passing this step will ensure us that there's a BitTorrent client on the other side. But that's not all we should require from the remote peer. 

You see, our current implementation downloads the file from a single peer, so the other requirement is that the peer we must have the entire file available. In general, that's not necessarily the case: in BitTorrent network the peers may have only parts of the file. Trying to download the file from such a peer would be a mistake in our current implementation: our client will just hang forever, waiting for missing file pieces. To avoid this misfortune, we should make sure that the peer has the entire file. 

Luckily, we can easily check the completeness of the file on the other end. Remember that right after the handshake, the remote peer sends us the `bitfield` message that contains the information about which pieces are available for download, in a form of a bitfield. All we need to do is to check that all pieces are present. That's going to be the second selection criterion for us. 

Next, to initiate the download, our client sends the `interested` message and expects to receive `unchoke` message in return. But that may never happen: the remote peer may never "unchoke" us, for reasons unknown. Currently, our client will just quit with the timeout error. However, there's a smarter strategy to handle this situation: instead of quitting, we can move on to the next peer in the list and request the download from it. Let's consider it a third selection criterion: the peer should send us `unchoke` message within a reasonable timeout. 

To summarize all the above, here's what we need to do when choosing the remote peer: 

1. Connect to the peer and exchange the handshake messages; 
2. Receive the `bitfield` message and make sure that the peer has the entire file; 
3. Send `interested` message and wait until it responds with `unchoke` message, or the timeout occurs. 

If any of these steps fail, we should move on to the next peer in the list. With some luck, eventually we'll come across the peer that satisfies all criteria. Otherwise, we can quit with a message that we were unlucky this time. 

# Extract the message exchange logic 

Until now, we had the initial message exchange implemented directly in [`main::download_file()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/bin/main.rs#L101) routine in a somewhat ad-hoc manner. It's time to give it a bit more attention and implement it properly. 

I extracted the message exchange logic into a separate function [`request_complete_file`](todo-link). This function implements steps #2 and #3 from above: 

* The sequence of messages didn't change: we still receive `bitfield` message, send `interested`, and wait for `unchoke` message from the other end; 
* The new functionality is that we now utilize the payload of the `bitfield` message: we analyze the contents of the bitfield and make sure that the remote peer contains the entire file. Otherwise, we return `IncompleteFile` error. 

Another notable change is the introduction of [`MessageChannel`](todo-link) trait to send and receive BitTorrent messages. This is an abstraction that makes `request_complete_file` testable. In production code, we want to send messages over the TCP channel to the remote host. In tests, however, we want to have an implementation of `MessageChannel` that allows us to simulate different scenarios that may occur in real life: incomplete files, unexpected bitfield data, etc. 

# Select the peer to work with 

TODO



# Refactor peer selection 

Removing `Peer` struct as redundant (use `SocketAddr` instead)

Probing the peer: 
  - Connect and handshake; 
  - Request the download; 

Creating the `request_download` function in `main`

Extracting the iteration logic into a higher-order function 






