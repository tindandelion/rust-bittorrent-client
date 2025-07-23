---
layout: post
title:  "Downloading the entire file"
date: 2025-07-23
---

Now that we managed to [download an verify a single piece][prev-post], extending the code to download the entire file is a quite straightforward progression. Since we know how many pieces there are, we can simply download them one by one, similar to how we were downloading a single piece in blocks. We're still going to be working under simplifications we [assumed previously][simplifications]. If we're lucky, we'll have the entire file downloaded by the end of this section, and gather more insights on the workings of BitTorrent protocol! 

# Implementation 

As I started to implement the whole file download, I had to make a choice where to put this functionality: 

1. We have a struct called [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader.rs#L21) that to some extent became a kitchen sink for methods related to peer communication. The name suggests that that could be the place for new functionality. 
2. Our other struct, [`PieceDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader/piece_downloader.rs#L13) does all the work related to downloading a single piece. We could extend its functionality to handle multiple pieces.
3. We could come up with a separate abstraction for handling the file download. 

Once I started coding, it turned out that the option #2 was the most convenient. On one hand, I already had a test suite for `PieceDownloader`, and it was easier to extend its functionality. On the other hand, there was not a lot of new code that would justify creating yet another abstraction on top of it. This situation can potentially change in the future when we start implementing more advanced features of BitTorrent protocol, but for our current requirements, `PieceDownloader` was the best option. 

Once the functionality was in place, the name of the struct, `PieceDownloader` was no longer adequate, so I renamed to reflect its main purpose, and now it's called [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/file_downloader.rs#L17). 

But wait, we already had a struct [`FileDownloader`][file-downloader-0.0.7], what about it? Well, it turns out that the name was a mistake. Over the course of development, this struct acquired methods that dealt more with low-level peer communication, rather than file downloading itself. So I re-purposed this struct to become [`PeerChannel`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/peer_channel.rs#L14) instead, and to be responsible for hiding the pesky details of peer communication on a level of individual messages. It also became a natural place to implement the [`DownloadChannel`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/peer_channel.rs#L14) trait. 

In a nutshell, we saw a _shift of responsibility_ between these two parts of the code. On one hand, the entity called `PieceDownloader` acquired a new natural responsibility and became a new `FileDownloader`. On the other hand, the former `FileDownloader` became a more focused `PeerChannel` struct. 

The overall structure of the program now looks like this: 

![File downloader structure]({{ site.baseurl }}/assets/images/download-the-whole-file/file-downloader-0.0.8.svg)

# Let's try it out! 

Let's now run our [`main`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/bin/main.rs#L10) program and see how it works. In addition, let's measure the time it takes to download the entire file with our current suboptimal implementation: 

```console

```



[file-downloader-0.0.7]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader.rs#L21