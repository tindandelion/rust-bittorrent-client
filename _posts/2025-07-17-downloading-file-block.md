---
layout: post
title:  "Downloading some data from a peer"
date: 2025-07-14
---

# Peer-to-peer communication 

In the traditional client-server request/response model the roles between the client and the server are quite well defined. When the client needs something, it makes a request to the server. The server does the work, and sends the response back to the client. If the server encounters an error during request execution, or is too busy handling other requests, it may reject the request by sending the error response. In any case, the client expects to receive something back. 

In contrast, the communication between peers in BitTorrent networks involves more ceremony. By its nature, it's more collaborative and inherently asynchronous. 

#### Announcing what we have 

It is implied that both peers can offer each other some file pieces for downloading, but at the beginning they don't know which pieces the other party has, so they first exchange the messages that describe what pieces they have to offer. They do it by exchanging the `bitfield` messages. The payload of those messages contains the bitfield that indicates which pieces the peer has available for download. 

It's also possible no notify other peers that we have some new pieces available for download. For example, when our client has finished downloading a new file piece, and verified its hash, it should send `have` message with the index of the newly downloaded piece to its peers. 


#### Expressing interest

Once the peer received this information from its counterpart, it can express interest in further communication by sending the `interested` message. That message notifies the other party that the peer would like to start requesting file blocks, so that it can do any necessary preparations. 

If for some reason the peer is no longer interested in downloading, it can send the `not interested` message to the remote peer. The remote part then can free up allocated resources. 

The communication always starts in `not interested` state on both sides. 

#### Choking my peer

It sounds criminal, but peers can also _choke_ each other. Choking means that the peer stops answering requests from the other side. To make it known to its counterpart, the peer sends it `choked` notification message. The client then should stop sending download requests to the peer, and consider all pending (sent but un-answered) requests to be discarded by the remote peer. 

The opposite action is _unchoking_, when the peer desides to allow its client make download requests. It notifies the client about it by sending `unchoke` notification message. 

At the beginning of the communication, both peers start in `choked` state. When one of them sends `interested` message to the other, that other can eventually allow downloading, and sends the `unchoke` message back. However, due to the capricious nature of peer-to-peer communication, there's no guarantee that we'll ever receive `unchoke` message in return. It's also possible that the remote peer can deside to choke us at any time. 

#### Download requests

Finally, when all formalities are done (the client sent `interested` message to the peer and received `unchoke` notification back), we can get to the matter: requesting file content for downloading. 

Download requests introduce one more layer of file fragmentation. Remember, we already split the file into _pieces_, calculate the SHA-1 hash of each piece, and store these hashes in the torrent file. Well, it turns out that it's not enough. When downloading the contents, we have to split pieces into even smaller _blocks_. As the [specification][bit-torrent-spec] notifies us, the state-of-the-art clients use block sizes no longer than 16KB. What it means for us is that to download the whole piece, we need to make multiple download requests of 16KB (or smaller) blocks. 






**You**: Hello! 
**They**: Hello. 

**They**: Here are the file blocks I can offer for download. 
**You**: Sure, I'm interested. 

**They**: I'm ready to listen to your requests. 

**You**: Can you give me 128 bytes of the piece index 0? 
**They**: Sure, here you go: 00 01 02 03...

# Peer message format 

All peer messages have the same format: 

![Peer message format]({{ site.baseurl }}/assets/images/downloading-file-block/peer-message-format.svg)

First 4 bytes are the message length (big-endian integer), followed by the message body goes after that. The body itself can be split into two parts: 1 byte for the message type id, and then the type-specific payload, which can have variable length. A format like that becomes quite handy when it comes to handling unknown messages. Since every message is prepended by its length, we can always skip over the unknown message by just reading the needed number of bytes from the TCP stream. 


