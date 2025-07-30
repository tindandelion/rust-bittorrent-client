---
layout: post
title:  "Request pipeline implementation"
date: 2025-07-30
---

So our [quick experiments][prev-post] have shown that request pipelining does in fact improve the download speed. Now we can move forward and create a proper implementation for it. 

[*Version 0.0.9 on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.9){: .no-github-icon}

# General considerations 

To implement request pipelining, we're going to break this tightly coupled loop in [`FileDownloader::download_piece_by_block()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/file_downloader.rs#L149): 

```rust 
fn download_piece_by_block(
    &mut self,
    piece_index: u32,
    piece_length: u32,
) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0; piece_length as usize];

    let block_count = piece_length.div_ceil(self.block_length);
    for block_index in 0..block_count {
        let (block_offset, block_length) =
            self.request_block(piece_index, block_index, piece_length)?;
        let data = self.receive_block(piece_index, block_offset, block_length)?;
        buffer[block_offset as usize..(block_offset + block_length) as usize]
            .copy_from_slice(&data);
    }
    Ok(buffer)
}
```

Basically, the pipelining algorithm works as follows: 

1. When the download starts, we send a series of `request` messages to the remote peer. The number of sent messages  defines the request queue length. 
2. Next we start waiting for `piece` messages from the peer. When a `piece` message is received, we issue the next `request` message. 
3. We repeat the step #2 in the loop until we receive all blocks. 

Also, we'd like the request pipeline to work across the piece boundaries. That means that once we've finished sending requests for the current piece, we immediately pick the next one and start requesting its blocks. In the first version, we'll just order the pieces by their indexes. 

The receiving algorithm also undergoes some changes. We're now working with the continuous stream of `piece` messages, decoupled from the corresponding requests: 

* We expect that `piece` messages for the same piece come in the pre-determined order. Each new incoming block must be a continuation of the previous one, without any gaps or overlaps. 
* Once we've received all blocks for a piece, we consider that piece finished, and start receiving the next piece. For the receiving algorithm, the order of pieces doesn't matter: we can detect the piece index from the `piece_index` field of the first received block. 


# Implementation details 

In order to facilitate testing, I've extracted two helper structs, [`RequestEmitter`][request-emitter] and [`PieceComposer`][piece-composer]. As their names suggest, they are responsible for sending `request` messages to the peer, and constructing the downloaded piece from incoming `piece` messages, respectively. 

#### _RequestEmitter_

[`RequestEmitter`][request-emitter] implements the algorithm for sending `request` messages to the peer, in a way described above. Internally, it keeps track of the current piece being requested, along with the next block inside that piece. Its method [`request_next_block()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader/request_emitter.rs#L22) does the bulk of work: 

* It calculates the parameters `block_offset` and `block_length` for the next block and calls `RequestChannel::request()`; 
* Once all blocks for the current piece have been requested, it increments the current piece index;
* When all pieces have been requested, it doesn't send any more requests and simply returns `Ok(())`. 

Its another method `request_first_blocks()` is supposed to be called when the download starts. It fills up the request pipeline by sending the first series of requests. The number of requests is determined by the parameter `n_requests`. 

#### _PieceComposer_

[`PieceComposer`][piece-composer] is responsible for reconstructing the piece from the incoming `piece` messages. Its main method [`append_block`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader/piece_composer.rs#L19) accepts the received file block and adds the block data to the current piece. If the appended block completes the current piece, `append_block` returns that piece as the result, and becomes ready to construct the next piece. Otherwise, it returns `None`. 

Additionally, `PieceComposer` verifies that blocks come in expected order: 
* The `piece_index` of the block must match the index of the currently constructed piece; 
* The `offset` of the block must be equal to (`offset + length` of the previous block). Essentially, it checks that data is received without any gaps or overlaps.  

#### _FileDownloader_ 

The new implementation of [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader.rs#L63) now relies on `RequestEmitter` and `PieceComposer` to do the lion share of the job. Its main method [`download()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader.rs#L94) ties all pieces together: 

```rust
pub fn download(&mut self) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0; self.file_info.file_length];
    let mut downloaded_pieces_count = 0;
    let mut download_report = DownloadReport::new();

    self.request_emitter
        .request_first_blocks(Self::REQUEST_QUEUE_LENGTH, self.channel)?;

    while downloaded_pieces_count < self.file_info.piece_count() {
        download_report.waiting_for_block();
        let block = self.channel.receive()?;
        self.request_emitter.request_next_block(self.channel)?;

        if let Some(piece) = self.piece_composer.append_block(&block)? {
            self.verify_piece_hash(piece.index, &piece)?;

            let (piece_start, piece_end) = self.file_info.piece_bounds(piece.index);
            buffer[piece_start..piece_end].copy_from_slice(&piece.data);

            download_report.piece_downloaded(piece.index);
            downloaded_pieces_count += 1;
        }
    }

    Ok(buffer)
}
```

# Choosing the request queue length 

Once the implementation is in place, the question becomes: how many requests should we send beforehand? In other words, how long should the request queue be? 

The [original paper](https://bittorrent.org/bittorrentecon.pdf) by Bram Cohen suggests that keeping 5 requests in the queue should "reliably saturate most connections". The [discussion page](https://wiki.theory.org/Talk_BitTorrentSpecification.html#Algorithms:_Queuing) of BitTorrent specification, however, challenged that assumption, claiming that with modern high-speed Internet the value of 5 is too low, and suggested values of 30 requests or more. 

I guess that value is a matter of trial and error in case of a static queue length. Let's try the queue length of 10 and see how much it improves the download speed in our local environment: 

```console
* Total pieces 2680, piece length 262144
* Connected to local peer: 127.0.0.1:26408
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
-- Receive: 499 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 498 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
- Downloaded piece 0: 1021 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 478 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 497 ms
-- Receive: 0 ms
- Downloaded piece 1: 1002 ms
```

The download speed has increased, but we still see 500 millisecond delays for every 10th piece. Let's increase the queue length to 20 then: 

```console 
* Total pieces 2680, piece length 262144
* Connected to local peer: 127.0.0.1:26408
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
-- Receive: 498 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
- Downloaded piece 0: 523 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 474 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
- Downloaded piece 1: 498 ms
```
That increased the download speed even more, but the delay is still there, only now it's every 20th request. 

I've played with different values for the queue length for a while and finally settled on the value of **150**. It looks like with that value we don't see 500 ms delays anymore, whereas bigger values didn't affect the download speed. 

With the queue length equal to 150 requests I finally managed to reach the peak download speed in the local environment: 

```console 
* Total pieces 2680, piece length 262144
* Connected to local peer: 127.0.0.1:26408
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
-- Receive: 498 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
- Downloaded piece 0: 522 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
-- Receive: 0 ms
- Downloaded piece 1: 12 ms

<< ...skipped... >> 

- Downloaded piece 2679: 5 ms
* Received entire file, first 128 bytes: 455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1
* File size: 702545920, download duration: 16.494491333s
```

Downloading the file in the local environment now takes 16 seconds, which gives us the download speed of almost **42 MB/sec**. What a dramatic change, compared to our [initial implementation][prev-download-speed]! 

#### My thoughts on the results

This experiment got me thinking. It looks like the length of the request queue is predicated on the channel bandwidth: the wider the channel, the longer the queue should be. We've implemented the static queue with a predefined length, which we chose by trial and error in a local environment. I guess there should be an algorithm that would adjust the length of the queue dynamically, adapting to the current channel bandwidth. 

On the other hand, this whole experiment was based on the single BitTorrent client implementation: Transmission. I hypothesized that there should be some forced delay in Transmission side, according to the irregularities in `piece` message delays. Other BitTorrent clients may behave differently. 

As a bottom line, I'd say that the static queue approach works fine for now, though its flexibility raises some concerns. Perhaps, later I could revisit the implementation, conduct some more experiments with various BitTorrent clients, and come up with a more flexible solution for request pipelining. 

# Trying it all out 

To conclude this part of work, let's remove all debug output from the code and see how request pipelining works with the remote peer from the wild: 

```console
* Total pieces 2680, piece length 262144

* Your announce url is: http://bttracker.debian.org:6969/announce
* Total 50 peers
* Probing peers...
87.58.176.238:62008	-> OK("-TR3000-0d2p8nysqd9z")
* Connected to peer: 87.58.176.238:62008
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
- Downloaded piece 0: 745 ms
- Downloaded piece 1: 65 ms
- Downloaded piece 2: 50 ms
- Downloaded piece 3: 18 ms
- Downloaded piece 4: 50 ms
- Downloaded piece 5: 10 ms
- Downloaded piece 6: 23 ms
- Downloaded piece 7: 8 ms
- Downloaded piece 8: 7 ms
- Downloaded piece 9: 55 ms
- Downloaded piece 10: 8 ms
- Downloaded piece 11: 54 ms
- Downloaded piece 12: 8 ms
- Downloaded piece 13: 7 ms
- Downloaded piece 14: 497 ms
- Downloaded piece 15: 61 ms
- Downloaded piece 16: 11 ms
- Downloaded piece 17: 9 ms

<< ...skipped... >> 

- Downloaded piece 2677: 6 ms
- Downloaded piece 2678: 384 ms
- Downloaded piece 2679: 48 ms
* Received entire file, first 128 bytes: 455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1
* File size: 702545920, download duration: 145.052801s
```

Obviously, the download speed from the remote peer was lower than in our local environment. But nonetheless, we've downloaded the entire file in roughly 2.5 minutes, with the download speed of **4.6 MB/sec**. Impressive improvement! 

# Next steps

So let's conclude for now that we achieved satisfactory download speeds with request pipelining, and switch our attention to another topic that needs some optimization: picking the peer to download from. 

[prev-post]: {{site.baseurl}}/{% post_url 2025-07-25-improve-download-speed %}
[request-emitter]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader/request_emitter.rs
[piece-composer]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.9/src/downloader/file_downloader/piece_composer.rs
[prev-download-speed]: {{site.baseurl}}/{% post_url 2025-07-23-download-the-whole-file %}#lets-try-it-out