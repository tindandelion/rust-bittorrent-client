---
layout: post
title:  "Downloading the entire file"
date: 2025-07-24
---

We've managed to [download and verify a single piece][prev-post]. After that, extending the code to download the entire file is quite a straightforward progression. Since we know how many pieces there are, we can simply download them one by one, similar to how we were downloading a single piece in 16Kb blocks. Only the last piece would require special care, because it can be shorter than the others. 

We're still going to be working under the simplifications we [assumed previously][simplifications]. If we're lucky, we'll have the entire file downloaded by the end of this section, and gather more insights into the workings of the BitTorrent protocol! 

[*Version 0.0.8 on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.8){: .no-github-icon}

# Implementation: the new _FileDownloader_

As I started to implement the whole file download, I had to make a choice about where to put this functionality: 

1. We have a struct called [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader.rs#L21) that to some extent has become a kitchen sink for methods related to peer communication. The name suggests that it could be the place for new functionality. However, it's now a bit of a mix of different levels of abstraction.  
2. Our other struct, [`PieceDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader/piece_downloader.rs#L13), does all the work related to downloading a single piece. We could extend its functionality to handle multiple pieces.
3. We could come up with a separate abstraction for handling the file download. This option makes sense if the implementation is complicated or requires additional collaborators. 

Once I started coding, it turned out that option #2 was the most convenient. On one hand, I already had a test suite for `PieceDownloader`, and it was easier to extend with the new functionality. On the other hand, there wasn't a lot of new code that would justify creating yet another abstraction on top of `PieceDownloader`. 

This situation can potentially change in the future when we start implementing more advanced features of the BitTorrent protocol, but for our current requirements, `PieceDownloader` looks like the best option to handle the new responsibility. 

Once the functionality was in place, the name of the struct, `PieceDownloader`, was no longer adequate, so I renamed it to reflect its new purpose, and now it's called [`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/file_downloader.rs#L17). 

But wait, we already had a struct [`FileDownloader`][file-downloader-0.0.7] before, what about it? Well, it turned out that naming it so was a mistake. Over the course of development, this struct acquired methods that dealt more with low-level peer communication, rather than the download logic itself. 

So I repurposed this struct to become [`PeerChannel`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/peer_channel.rs#L14) instead, and to be responsible for hiding the pesky details of peer communication at the level of individual messages. It also became a natural implementor of the [`DownloadChannel`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/downloader/peer_channel.rs#L14) trait used by the `FileDownloader`. 

In a nutshell, we saw a _shift of responsibility_ between these two parts of the code. On one side, the entity called `PieceDownloader` acquired a new natural responsibility and became a new `FileDownloader`. On the other side, we noticed that the former `FileDownloader` was mostly dealing with peer communication at the lower level of abstraction, so we changed it to become a more cohesive `PeerChannel` struct. 

This is a natural process in _evolutionary design:_ you constantly monitor the structure of the program as you add more and more features. When necessary, you refine your existing design to accommodate new responsibilities in the most natural way. You can do that fearlessly because the tests you wrote along with the code help you make changes without breaking the code. As a result, the design continuously _evolves_ as you move forward and discover better ways to structure the program. 

After all the changes, the overall structure of the program started to look like this: 

![File downloader structure]({{ site.baseurl }}/assets/images/download-the-whole-file/file-downloader-0.0.8.svg)

# Let's try it out! 

Let's now run our [`main`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.8/src/bin/main.rs#L10) program and see how it works. In addition, let's measure the time it takes to download the entire file with our current simplified implementation: 

```console
* Total pieces 2680, piece length 262144

* Your announce url is: http://bttracker.debian.org:6969/announce
* Total 50 peers
* Probing peers...
202.187.152.10:63726	-> Err(failed to fill whole buffer)
185.203.56.59:61635	-> OK("-lt0D80-������}�\u{5}���")
* Connected to peer: 185.203.56.59:61635
* Received bitfield: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
* Sending `interested` message
* Receiving `unchoke` message
* Unchoked, requesting file
Downloading piece 0
Downloading piece 1
...
Downloading piece 2678
Downloading piece 2679
* Received entire file, first 128 bytes: 455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1
* File size: 702545920, download duration: 6641.095567541s
```

Wow, it took quite a while to download the entire file! The program ran for almost 2 hours, for a file of approximately 670 megabytes. That gives us a download speed of roughly 103 Kb/sec. Not much! 

#### Why is the download so slow? 

This is a subject for my future experiments, but at least 3 possible reasons come to mind: 

1. The peer can have a slow connection or experience high load; 
2. The peer can limit the download speed in its settings; 
3. Our download algorithm is not optimal. 

Of those, only the last option is under our control, so we can focus on improving the download process on our side of the fence. But before jumping to solutions, I'd like to run some experiments to collect more data to better understand where the bottleneck might be. 

This is going to be a subject for my next step.

[prev-post]: {{site.baseurl}}/{% post_url 2025-07-19-download-the-whole-piece %}
[simplifications]: {{site.baseurl}}/{% post_url 2025-07-19-download-the-whole-piece %}#some-wild-assumptions
[file-downloader-0.0.7]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.7/src/downloader.rs#L21