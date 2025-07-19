---
layout: post
title:  "Downloading the whole piece"
date: 2025-07-18
---

Now that we're able to [download a portion of the file][prev-post] from the remote peer, I'm tempted to milk this cow dry. Let's now download the entire file piece and verify its validity by checking its SHA-1 hash. 

# Some wild assumptions

For the first version, I'm going to make a few important simplifications to the download algorithm: 

1. We'll assume that, once we've performed all required ceremonial message exchanges with the remote peer, the rest of the communication will be us sending `request` messages and them responding with `piece` messages back. No other messages will be expected. In this implementation, we'll just panic in case we receive a message of another type from the remote peer.
2. To download the piece, we'll be doing the full round-trip for each data block: we'll ask for a block and wait until it arrives, one at a time. This is not an efficient strategy in terms of bandwidth utilization: BitTorrent specification suggests that the optimal strategy is to pipeline download requests. 

These are rather gross simplifications that we'll need to address in the future. While #2 looks like a pure performance optimization, it is going to be crucial to address #1 eventually: we can't assume that we're only going to receive `piece` messages from the remote peer. At the very least, we'll need to process `choke` and `unchoke` messages as well, since they directly affect the download. But for now, I'd like to pretend that we're never going to be choked. 

Let's see how it plays out. 

# Implementation 

With these assumptions in mind, the algorithm for downloading the entire piece becomes fairly trivial: 

* We know the length of the single piece: it comes from the torrent file in the field `piece length` of the [`info` dictionary][bt-spec-info-dict]. For our sample torrent file, the piece length is 262144 bytes; 
* Once we know the piece length, we simply download it in 16KB blocks, one block at a time, by sending the `request` message to the peer and waiting for the `piece` message in response. 

This algorithm is implemented in the [`PieceDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/downloader/piece_downloader.rs#L13) struct, with the help of an intermediate abstraction `PieceDownloadChannel`: 

![Peer message format]({{ site.baseurl }}/assets/images/download-the-whole-piece/piece-downloader.svg)

I've created the trait `PieceDownloadChannel` to facilitate testing: in tests, we provide fake implementations of this trait to simulate successful and a few failure scenarios. 

In particular, I consider it a failure if the peer responds with a `piece` message whose `offset` or `length` field is different from what we requested. Technically, the BitTorrent protocol message format doesn't prevent such situations. However, I don't know if that's the case in production. In any case, it's better to detect them early on. 

Finally, the new method [`FileDownloader::download_piece()`](https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/downloader.rs#L71) utilizes `PieceDownloader` to do the job. 

# Verifying the downloaded piece 

The `.torrent` file contains one more important bit of data about file pieces: SHA-1 hash of each piece. Using the hashes, the client can verify the downloaded piece content. SHA-1 hashes are stored in the `pieces` field of the [`info` dictionary][bt-spec-info-dict] in the `.torrent` file. That string field is actually a concatenation of all piece hashes, so in order to use them, we first need to split this string into 20-byte chunks (the length of SHA-1 hash). 

So, as a final touch of our piece downloading exercise, we use the piece hash from the torrent file to verify the downloaded content and print the final verdict to the console. 

# Let's give it a try! 

Let's run our updated [`main` routine](https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/bin/main.rs#L9) and observe the output: 

```console
[main] $ cargo run --quiet
* Total pieces 2680, piece length 262144

* Your announce url is: http://bttracker.debian.org:6969/announce
* Total 50 peers
* Probing peers...
77.33.175.70:51413      -> Err(connection timed out)
107.159.249.132:59501   -> OK("-lt0D80-7\u{c}y��n�YZ��\u{11}")
* Connected to peer: 107.159.249.132:59501
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting data block
* Received piece: 455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1
* DOWNLOADED PIECE MATCHES EXPECTED HASH
[main] $ 
```

It took a couple of seconds in this run for our program to download the piece. And, the content matches the hash from the torrent file. Bingo! 

# Next steps 

Since we have successfully downloaded a single piece, it's only natural to capitalize on this success. I think the next step is going to be downloading the entire file! 

[prev-post]: {{site.baseurl}}/{% post_url 2025-07-17-downloading-file-block %}
[bt-spec-info-dict]: https://wiki.theory.org/BitTorrentSpecification#Info_Dictionary