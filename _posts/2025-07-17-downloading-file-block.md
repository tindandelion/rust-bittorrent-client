---
layout: post
title:  "Downloading some data from the peer"
date: 2025-07-14
---

[Last time][prev-post] we left off being able to exchange the initial handshake messages with some of the peers. By the end of the session, I had some doubts about what to do next: should I continue working on peer-to-peer communication, or should I elaborate on the code that we already had working? After some reflection, I realized that peer communication was more interesting to move forward with. 

It's a bit of a diversion from my [original plan][original-plan], but I'm eager to take this detour. You see, I still have no idea how to _actually_ download a piece of the file from the remote peer. That makes me anxious: since I don't know what it's going to take, I'm very uncomfortable about the design decisions I need to make _right now_. I hope that if I advance far enough towards the file download, I'll understand the mechanics of peer communication better, and eventually I'll be better informed about how to structure the program.

So let's spearhead a little bit in this direction. The goal of this section is to be able to download at least a single byte of the file from the remote peer. Let's do it! 

# Peer message format 

Once the TCP connection is established and peers have exchanged the handshake messages, they are ready to start collaborating. The communication is bi-directional and symmetrical, meaning that the messages sent in both directions look the same, and the data flow can go in either direction. On a low level, all peer messages [have the same format](https://wiki.theory.org/BitTorrentSpecification#Messages): 

![Peer message format]({{ site.baseurl }}/assets/images/downloading-file-block/peer-message-format.svg)

The first 4 bytes are the message length (big-endian integer), followed by the message body. The body itself can be split into two parts: 1 byte for the message type id, and then the type-specific payload, which can have variable length. 

A format like that becomes quite handy when it comes to handling unknown messages. Since every message starts with the length, we can always skip over an unknown message by just reading the needed number of bytes from the TCP stream. 

It's also worth mentioning that all integer fields are encoded in **4-byte big-endian format**. _Endianness_ is very important when it comes to how multi-byte integers are represented in memory or in the data stream. It determines the order in which bytes appear in the data stream. _Big-endian_ means that the most significant byte comes first, e.g.: 

```
305419896: (0x12345678): 0x12 0x34 0x56 0x78                    
```

Big-endian byte order is quite common in network protocols. Little-endian order is more prominent in various processor architectures.  

# Peer-to-peer communication flow

Compared to the traditional client-server request/response model, the communication between peers in BitTorrent networks involves more ceremony. By its nature, it's more collaborative and inherently asynchronous. Peers can express interest in each other's data, notify each other when they start or stop accepting download requests, etc. The whole communication happens asynchronously, meaning that any message can come at any time. 

For the purposes of this section, however, we're going to assume a simpler view of the communication. Remember, our goal right now is to persuade the remote peer to send us a block of the file data. On the high level, the message exchange will resemble the following conversation: 

> **You:** Hello!<br>
> **They:** Hello. 
>
> **They:** Here are the file blocks I can offer for download.<br> 
> **You:** I'm interested!
> 
> **They:** I'm ready to listen to your requests. 
>
> **You:** Can you give me 128 bytes of piece index 0?<br> 
> **They:** Sure, here you go:  00 01 02 03 ...

Let's see how this dialog maps to the peer messages. 

#### Announcing what we have 

It is implied that both peers can offer each other some file pieces for downloading, but at the beginning they don't know which pieces the other party has. They first exchange the messages that describe what pieces they have to offer, by exchanging the [`bitfield(id=5)`](https://wiki.theory.org/BitTorrentSpecification#bitfield:_.3Clen.3D0001.2BX.3E.3Cid.3D5.3E.3Cbitfield.3E) messages. The payload of those messages contains the bitfield that indicates which pieces the peer has available for download. 

It's also possible to notify other peers that we have some new pieces available for download. For example, when our client has finished downloading a new file piece and verified its hash, it should send a [`have(id=4)`](https://wiki.theory.org/BitTorrentSpecification#have:_.3Clen.3D0005.3E.3Cid.3D4.3E.3Cpiece_index.3E) message with the index of the newly downloaded piece to its peers. 


#### Expressing interest

Once the peer has received that information from its counterpart, it can express interest in further communication by sending the [`interested(id=2)`](https://wiki.theory.org/BitTorrentSpecification#interested:_.3Clen.3D0001.3E.3Cid.3D2.3E) message. That message notifies the other party that the client would like to start requesting file blocks, so that the peer can do any necessary preparations. 

If for some reason the peer is no longer interested in downloading, it should send the [`not interested(id=3)`](https://wiki.theory.org/BitTorrentSpecification#not_interested:_.3Clen.3D0001.3E.3Cid.3D3.3E) message to the remote peer. The remote peer then can free up allocated resources. 

The communication always starts in the `"not interested"` state on both sides. 

#### Choking my peer

It sounds criminal, but peers can also _choke_ each other. Choking means that the peer stops answering requests from the other side (i.e. it chokes its partner). To make it known to its counterpart, the peer sends a [`choke(id=0)`](https://wiki.theory.org/BitTorrentSpecification#choke:_.3Clen.3D0001.3E.3Cid.3D0.3E) notification message. The client then should stop sending download requests to the peer and consider all pending (sent but unanswered) requests to be discarded by the remote peer. 

The opposite action is _unchoking_, when the peer decides to allow its client to make download requests. It notifies the client about it by sending an [`unchoke(id=1)`](https://wiki.theory.org/BitTorrentSpecification#unchoke:_.3Clen.3D0001.3E.3Cid.3D1.3E) notification message. 

At the beginning of the communication, both peers start in the `choked` state. When one of them sends an `interested` message to the other, that other can eventually allow downloading and send the `unchoke` message back. However, due to the capricious nature of peer-to-peer communication, there's no guarantee that we'll ever receive an `unchoke` message in return. 

It's also possible that the remote peer decides to choke us at any time. 

#### Download requests

Finally, when all formalities are done (the client sent an `interested` message to the peer and received an `unchoke` notification back), we can get to the matter: requesting file content for downloading. 

Download requests introduce one more layer of file fragmentation. Remember, we already split the file into _pieces_, calculate the SHA-1 hash of each piece, and store these hashes in the torrent file. Well, it turns out that it's not enough. When downloading the contents, we have to split pieces into even smaller _blocks_. As the [specification][bit-torrent-spec-block-size] informs us, the state-of-the-art clients use block sizes no longer than 16KB. What it means for us is that to download the whole piece, we need to make multiple download requests of 16KB (or smaller) blocks. 

To request the file block, we should send the [`request(id=6)`](https://wiki.theory.org/BitTorrentSpecification#request:_.3Clen.3D0013.3E.3Cid.3D6.3E.3Cindex.3E.3Cbegin.3E.3Clength.3E) message. In this message, we provide the piece index we are interested in, the byte offset within the piece, and the length of the requested block. 

If we're lucky, the peer will eventually send us the requested block in the [`piece(id=7)`](https://wiki.theory.org/BitTorrentSpecification#piece:_.3Clen.3D0009.2BX.3E.3Cid.3D7.3E.3Cindex.3E.3Cbegin.3E.3Cblock.3E) message. 

# Implementing the basic flow to receive a file block

As you can see, there's quite a bit of chit-chat happening between peers before we can actually receive the block of file data. Fortunately, to get things going, we can strip this communication to the bare minimum and pretend as if it were a linear message exchange: 

1. Receive `bitfield` message from the peer; 
2. Send `interested` message back; 
3. Receive `unchoke` notification; 
4. Send a `request` message to request the very first 128 bytes of the file (piece index = 0, byte offset = 0, length = 128)
5. Receive `piece` message with the requested file block. 

This is a very simplified view of peer-to-peer communication. In real life, there's no guarantee this communication will go so smoothly. For example, the connected peer may not have any pieces for download, so it won't send us the `bitfield` message. Or, it may be busy at the moment, so we won't ever receive the `unchoke` notification. Nonetheless, I'm feeling lucky! 

#### _PeerMessage_ enum

To represent different peer messages, I've created the [`PeerMessage`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/src/downloader/peer_messages.rs#L4) enum. As the name suggests, this enum represents different kinds of peer messages. Functionality-wise, `PeerMessage` can construct itself from `io::Read` and write its contents into `io::Write`. Essentially, I've written a bespoke serialization/deserialization mechanism that complies with the [peer message format](#peer-message-format).

Since currently we only need a subset of messages, and not all of them need to be sent or received, I've provided only the bare minimum of serialization/deserialization, deferring the rest to the future. I'd also like to explore the options of using third-party libraries to perform the serialization because that code is pretty boring to write. 

#### Connecting to the peer 

Building upon the previous functionality, we find the first peer that responds to the handshake with the help of the [`connect_to_first_available_peer`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/src/bin/main.rs#L62) function: 

```rust
fn connect_to_first_available_peer(
    peers: &[Peer],
    info_hash: Sha1,
    peer_id: PeerId,
) -> Option<FileDownloader> {
    for peer in peers {
        print!("{}:{}\t-> ", peer.ip, peer.port);
        match probe_peer(&peer, info_hash, peer_id) {
            Ok((result, downloader)) => {
                println!("OK({})", result);
                return Some(downloader);
            }
            Err(e) => println!("Err({})", e),
        }
    }
    None
}
```

Peer discovery is still done **synchronously**: we try to connect to the peers one by one, waiting for them to respond with a timeout of 5 seconds. This is absolutely not the solution you'd like to see in production: if the first 30 peers happen to be unresponsive, it would take us 30 * 5 = 600 seconds until we finally encounter the first responsive peer! I'm going to need to do something about it. Probing peers in parallel looks like a viable solution. 

#### _download_file_block()_ function 

The [`download_file_block()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/src/downloader/peer_messages.rs#L4) function in `main.rs` quite literally implements the communication flow from [above](#implementing-the-communication-flow) to request first `N` bytes from the beginning of the data file (piece index = 0, byte offset = 0): 

```rust
fn download_file_block(
    downloader: &mut FileDownloader,
    block_length: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let bitfield = downloader.receive_bitfield()?;
    println!("* Received bitfield: {}", hex::encode(bitfield));

    println!("* Sending `interested` message");
    downloader.send_interested()?;

    println!("* Receiving `unchoke` message");
    downloader.receive_unchoke()?;

    println!("* Unchoked, requesting data block");
    downloader.request_block(0, 0, block_length)?;

    println!("* Receiving `piece` message");
    let (piece_index, offset, block) = downloader.receive_piece()?;
    println!(
        "* Received block of piece {} at offset {}: {} ",
        piece_index,
        offset,
        hex::encode(block.clone())
    );
    Ok(block)
}
```
To make the code look smoother, I've added a few helper methods to the [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/src/downloader.rs#L19) interface. They are essentially just wrappers around `PeerMessage` send/receive functionality, with additional checks that the correct message type was received. 

#### Checking the result 

Finally, I wanted to make sure that we receive the correct file block. In order to do that, I've put a test data file into the [`test-data`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/test-data/debian-12.11.0-amd64-netinst-part.iso) directory. Essentially, it's the first 16KB of the original Debian ISO file that we're supposed to download. 

Once we receive the file block from the peer, we can compare the contents of that block with the test file. If they match, that means we managed to download a part of the file successfully!

# Give it a try! 

Let's now run our updated [`main`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.6/src/bin/main.rs#L9) routine and see if we can communicate with a peer that cares to respond:

```console
[main] $ cargo run --quiet
* Total pieces 2680, piece length 262144

* Your announce url is: http://bttracker.debian.org:6969/announce
* Total 50 peers
* Probing peers...
85.134.8.12:51413       -> OK("-TR3000-ev59bulyuscr")
* Connected to peer: 85.134.8.12:51413
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting data block
* Receiving `piece` message
* Received block of piece 0 at offset 0: 455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1 


* RECEIVED BLOCK MATCHES SAMPLE DATA
[main] $
```
I got lucky this time, and the very first peer in the list accepted the connection. It's also fortunate that the peer happens to have the entire file, as we can see from the received bitfield (all bytes are `0xff`, meaning that it has all file pieces). 

Next we went through the entire message flow and managed to receive the file block back. Finally, our sanity check passed: the received file block matches the beginning of the test data file! **Hooray!**

# What I've learned so far 

This section was a deeper dive into the BitTorrent protocol. Let's briefly summarize what we've learned so far: 

* Peer protocol is symmetrical and asynchronous: the same messages go in both directions; 
* Peers start the communication in `choked` and `not_interested` states; 
* To start requesting file blocks, the client should send an `interested` message to the peer and wait until it receives an `unchoke` notification from the peer; 
* Downloads happen in blocks no longer than 16KB in length. Each file piece (in terms of the torrent file) is longer than 16KB, so multiple download requests are needed to download the whole piece. 


[prev-post]: {{site.baseurl}}/{% post_url 2025-07-11-handshake-with-peers %}
[original-plan]: {{site.baseurl}}/{% post_url 2025-06-26-intermediate-reflection %}#the-plan
[bit-torrent-spec-block-size]: https://wiki.theory.org/BitTorrentSpecification#request:_.3Clen.3D0013.3E.3Cid.3D6.3E.3Cindex.3E.3Cbegin.3E.3Clength.3E
