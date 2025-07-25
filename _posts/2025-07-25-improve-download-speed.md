---
layout: post
title:  "Speeding up the download"
date: 2025-07-24
---

TODO: Small summary 

# Local peer setup 

It dawned on me that we can greatly simplify our experiments if we set up the BitTorrent client locally, and connect to it directly. After all, all we need to know is the IP address of the peer and the port on which it's listening for incoming requests. By running it locally we have the full control over the peer settings and network. 

I'm using [Transmission] for MacOS as a reference client. I've registered our `.torrent` file in it, and waited until it finished downloading. So now we have a BitTorrent client running locally that serves the entire file: 

![Transmission main window]({{ site.baseurl }}/assets/images/improve-download-speed/transmission-main-window.png)

The second thing is the port number on which it's listening for incoming connections. It can be specified in Transmission's settings, like that: 

![Transmission settings]({{ site.baseurl }}/assets/images/improve-download-speed/transmission-settings.png)

Now I should be able to connect to the local BitTorrent peer at the address `localhost:26408`. To run the experiments locally, I've created a separate binary crate called [`local_peer`][local-peer]. Essentially, I just copied the contents of the `main` crate, and removed everything related to the communication with the torrent tracker. It simply connects to the peer at the known address:

```rust
const LOCAL_PEER_PORT: u16 = 26408;

fn connect_to_local_peer(info_hash: Sha1, peer_id: PeerId) -> Result<PeerChannel, Box<dyn Error>> {
    let socket_addr = format!("127.0.0.1:{}", LOCAL_PEER_PORT)
        .to_socket_addrs()?
        .next()
        .unwrap();
    let mut channel = PeerChannel::connect(&socket_addr)?;
    channel.handshake(info_hash, peer_id)?;
    Ok(channel)
}
```

# Request timing 

With the local setup like this, we can start digging deeper into what's going with the download speed. The first experiment to make is to measure the time it takes to: 

* send the `request` message to the peer; 
* receive the corresponding `piece` response. 

Let's do it, printing the times to the console: 

```console
[main] $ cargo run --quiet --bin local_peer
* Total pieces 2680, piece length 262144
* Connected to local peer: 127.0.0.1:26408
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
- Downloading piece 0
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 500 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 500 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 500 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 500 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
- Downloaded piece 0, time: 8036 ms
- Downloading piece 1
-- Requesting block: 0 ms
-- Receiving block: 463 ms
^C
[main] $ 
```

That's a very interesting result! As we can see, the `request` message is sent instantaneously, but we wait for around 0.5 seconds to receive the `piece` message in response. And it happens for each block, resulting in 8 seconds to receive the entire piece! Also, remarkably, the delay is fairly constant around 500 milliseconds. It suggests that Transmission implements some sort of a forced delay on its end: it's highly improbable that downloading 16Kb of data via local connection could take almost half a second. 

Is there anything we can do about it? Obviously, we can't change the Transmission's behaviour, but there's one trick we can try on our side: _pipelining requests_

# An experiment with pipelining requests 

While figuring out the download rate issues, I came across the document written by Bram Cohen himself, the author of the BitTorrent protocol: [Incentives Build Robustness in BitTorrent](https://bittorrent.org/bittorrentecon.pdf). In Section 2.3 this document discusses the importance of _request pipelining_ to achieve good download rates: 

> When transferring data over TCP, like BitTorrent does, it is very important to always have several requests pending at once, to avoid a delay between pieces being sent, which is disastrous for transfer rates. BitTorrent facilitates this by breaking pieces further into sub-pieces over the wire, typically sixteen kilobytes in size, and always keeping some number,typically five, requests pipelined at once. Every time a sub-piece arrives a new request is sent. The amount of data to pipeline has been selected as a value which can reliably saturate most connections.

The BitTorrent specification [also emphasizes](https://wiki.theory.org/BitTorrentSpecification#Queuing) queuing requests for good download rates, although it suggests keeping 10 requests in the queue, as opposed to 5 in the original paper.  

So it looks like pipelining (or queuing) requests is a key to improving download rate. Let's run a quick'n'dirty experiment to prove that it's true. 

In this experiment, we'll request _all blocks for the piece at once at the beginning_, and then we'll just collect incoming `piece` messages, until all of them have arrived, by making a quick change to the code: 

```rust 
fn download_piece_by_block(
    &mut self,
    piece_index: u32,
    piece_length: u32,
) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0; piece_length as usize];

    let block_count = piece_length.div_ceil(self.block_length);

    // Send the requests for all blocks
    for block_index in 0..block_count {
        self.request_block(piece_index, block_index, piece_length)?;
    }

    // Receive the blocks
    for _ in 0..block_count {
        let block = self.receive_block()?;
        let block_offset = block.offset as usize;
        let block_length = block.data.len();
        buffer[block_offset..(block_offset + block_length)].copy_from_slice(&block.data);
    }
    Ok(buffer)
}
```

Running this version, we get the following output in the console: 

```console
[main] $ cargo run --quiet --bin local_peer
* Total pieces 2680, piece length 262144
* Connected to local peer: 127.0.0.1:26408
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
- Downloading piece 0
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Receiving block: 499 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
- Downloaded piece 0, time: 536 ms
- Downloading piece 1
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Requesting block: 0 ms
-- Receiving block: 463 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
-- Receiving block: 0 ms
- Downloaded piece 1, time: 500 ms
^C
[main] $ 
```

Wow, that's a dramatic difference! Now we waste no time waiting for file blocks to arrive, it happens instantly. The only time we still experience the delay is when receiving the very first block of a piece, again waiting for 500 milliseconds. I bet we can eliminate this delay as well if we also pipeline the requests across the piece boundaries! 

That experiment has shown us that request pipelining is definitely a way to go. Now let's revert all experimental changes and focus on the proper implementation of this algorithm. 