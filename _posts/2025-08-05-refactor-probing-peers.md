---
layout: post
title:  "Refactor probing peers"
date: 2025-08-05
---

TODO: Small summary 

# Choose the peer to download from 

Let's think about which peer we should pick for downloading the file. First and foremost, obviously, we should be able to connect to that peer and exchange the handshake messages. That's the bare minimum: passing this step will ensure us that there's a BitTorrent client on the other side. But that's not all we should require from the remote peer. 

You see, in our current implementation downloads the file from a single peer, so the other requirement is that the peer we connect to must have the entire file available. That's not the case, in general: in BitTorrent network the peers may have only parts of the file. Trying to download the file from such a peer would be a mistake. In our current implementation, our client will just hang forever, waiting for missing file pieces. To avoid this misfortune, we should make sure that the peer has the entire file. 

Luckily, we can easily check the completeness of the file on the other end. Remember that right after the handshake, the remote peer sends us the `bitfield` message that contains the information about which pieces are available for download, in a form of a bitfield. All we need to do is to check that all pieces are present. That's going to be the second selection criterion for us. 

Next, to initiate the download, our client sends the `interested` message and expects to receive `unchoke` message in return. But that may never happen: the remote peer may never "unchoke" us, for reasons unknown. In our current implementation, our code will just fail with the timeout error. However, there's a smarter strategy to handle this situation: instead of quitting, we can move on to the next peer in the list and request the download from it. Let's consider it a third selection criterion: the peer should send us `unchoke` message within a reasonable timeout. 

To summarize all the above, here's what we need to do when choosing the remote peer: 

1. Connect to the peer and exchange the handshake messages; 
2. Receive the `bitfield` message and make sure that the peer has the entire file; 
3. Send `interested` message and wait until it responds with `unchoke` message, or the timeout occurs. 

If any of these steps fail, we should move on to the next peer in the list. With some luck, eventually we'll come across the peer that satisfies them all. Otherwise, we can quit with a message that we were unlucky this time. 






# Request download procedure 

* More strict criteria: 
    - The remote peer must have the entire file (check bitfield); 
    - Sending `interested` event and receiving `unchoke`; 
    - Unexpected messages during request 

* Refactor the whole interaction into `request_complete_file` function: 
  - `MessageChannel` trait 

# Refactor peer selection 

Removing `Peer` struct as redundant (use `SocketAddr` instead)

Probing the peer: 
  - Connect and handshake; 
  - Request the download; 

Creating the `request_download` function in `main`

Extracting the iteration logic into a higher-order function 






