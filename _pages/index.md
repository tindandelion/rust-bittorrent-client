---
# Feel free to add content and custom Front Matter to this file.
# To modify the layout, see https://jekyllrb.com/docs/themes/#overriding-theme-defaults

layout: home
title: Welcome 
permalink: /    
list_title: Project diary
---

As I'm [continuing my programming adventures in Rust][first-rust-project], I decided to launch yet another learning project. This time, it's going to be an implementation of a simple [BitTorrent][bit-torrent] client. 

### Motivations

Why BitTorrent? One reason is that I've been interested in how peer-to-peer systems work. There's something very intriguing in how multiple actors can collaborate to accomplish some task without the need for centralized control. By diving deeper into the implementation of one of such systems, I hope to get more understanding in how they work in general. 

Another reason is that I expect that a project like that would help me deepen my experience with Rust. In particular, I would like to become more famililiar with these areas that I haven't touched yet: 

* Network programming; 
* Multi-threaded programming; 
* Programming console UIs. 

### Project scope 

Writing a fully-fledged BitTorrent client is quite a big task, so for my pet project I'd like to scale it down to the essentials. I will consider the project accomplised when my solution is able to do the following: 

* Connect to the torrent tracker to fetch the initial information about the file to download; 
* Download the file from multiple peers in parallel; 
* Serve requests from other peers while the download is ongoing; 
* Show the download progress in some form of a text-based UI. 

It should be noted, that I'm starting this project knowing _absolutely nothing_ about the BitTorrent protocol. Of course, I have some experience with various BitTorrent clients _as a user_, but I have absolutely no idea how they work under the hood. But hey, that's what this project is all about: getting into the nitty-gritty details. 

**Let's get going!**

[first-rust-project]: https://www.tindandelion.com/rust-text-compression/
[bit-torrent]: https://www.bittorrent.com/