---
layout: post
title:  "Request pipeline implementation"
date: 2025-07-30
---

So our [quick experiments][prev-post] have shown that request pipelining does in fact improve the download speed. Now we can move forward and create a proper implementation for it. 

# General considerations 

To implement request pipelining, we're going to break this tightly coupled loop in [FileDownloader::download_piece_by_block()](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/file_downloader.rs#L149): 

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

Our new algorithm works along the following general structure: 


1. When the download starts, we'll send a bunch of `request` messages to the remote peer. The number of messages sent essentially defines the length of the request queue. 
2. Next we start waiting for `piece` messages from the peer. Once each `piece` message is received, we submit the next `request` message. 
3. We repeat the step #2 in the loop until we receive all blocks. 

Also, we'd like the request pipeline to work across the piece boundaries. That means that once we've finished sending requests for the current piece, we immediately pick the next one. In the first version, we'll just be requesting pieces the order of their indexes. 

The receiving algorithm also undergoes some changes. We're now working with the stream of `piece` messages: 

* We expect that `piece` messages for the same piece come in the pre-determined order. Each new incoming block must be a continuation of the previous one, without any gaps or overlaps. 
* Once we've received all blocks for a piece, we consider that piece finished, and start composing the next piece. 


# Implementation details 

In order to facilitate testing, I've extracted two helper structs, [`RequestEmitter`]() and [`PieceComposer`](). As their names suggest, they are responsible for sending `request` messages to the peer, and constructing the downloaded piece from incoming `piece` messages, respectively. 

#### _RequestEmitter_

`RequestEmitter` implements the algorithm for sending `request` messages to the peer, in a way described above. Internally, it keeps track of the current piece being requested, along with the next block inside that piece. Its method [`request_next_block()`]() does the bulk of work: 

* It calculates the parameters `block_offset` and `block_length` for the next block and calls `RequestChannel::request()`; 
* Once all blocks for the current piece have been requested, it increments the current piece index;
* When all pieces have been requested, it doesn't send any more requests and simply returns `Ok(())`. 

Its another method `request_first_blocks()` is supposed to be called when the download starts. It fills up the request pipeline by sending the first series of requests. The number of requests is determined by the parameter `n_requests`. 

#### _PieceComposer_

`PieceComposer` is responsible for reconstructing the piece from the incoming `piece` messages. Its main method [`append_block`] accepts the received file block and adds the block data to the current piece. If the appended block completes the current piece, `append_block` returns that piece as the result, and becomes ready to construct the next piece. Otherwise, it returns `None`. 

In addition, `PieceComposer` verifies that blocks come in expected order: 
* The `piece_index` of the block must match the index of the currently constructed piece; 
* The `offset` of the block must be equal to (`offset + length` of the previous block). Essentially, it checks that data is received without any gaps or overlaps.  

#### _FileDownloader_ 

The new implementation of [`FileDownloader`]() now relies on `RequestEmitter` and `PieceComposer` to do the lion share of the job. Its main method [`download()`]() ties all pieces together: 

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

# Choosing request queue length 

# Trying it all out 

# Next steps
