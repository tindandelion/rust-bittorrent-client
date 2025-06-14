---
# Feel free to add content and custom Front Matter to this file.
# To modify the layout, see https://jekyllrb.com/docs/themes/#overriding-theme-defaults

layout: home
title: Welcome 
permalink: /    
list_title: Project diary
---

As I [continue my programming adventures in Rust][first-rust-project], I've decided to launch yet another learning project. This time, it's going to be an implementation of a [simple BitTorrent client][project-github]. 

### Motivations

Why BitTorrent? One reason is that I've always been interested in how peer-to-peer systems work. There's something very intriguing about how multiple actors can collaborate to accomplish a task without the need for centralized control. By diving deeper into the implementation of such a system, I hope to gain a better understanding of how they work in general. 

Another reason is that I expect a project like this will help me deepen my experience with Rust. In particular, I would like to become more familiar with these areas that I haven't explored yet: 

* Network programming
* Multi-threaded programming
* Programming console UIs

### Project scope 

Writing a fully-fledged BitTorrent client is quite a big task, so for my pet project I'd like to scale it down to the essentials. I will consider the project accomplished when my solution is able to do the following: 

* Connect to the torrent tracker to fetch the initial information about the file to download
* Download the file from multiple peers in parallel
* Serve requests from other peers while the download is ongoing
* Show the download progress in some form of text-based UI

It should be noted that I'm starting this project knowing _absolutely nothing_ about the BitTorrent protocol. Of course, I have some experience with various BitTorrent clients _as a user_, but I have absolutely no idea how they work under the hood. But hey, that's what this project is all about: getting into the nitty-gritty details!

### Useful links 

Obviously, I'm not the first person to implement a BitTorrent client. There are plenty of resources on the Web dedicated to this subject. Here, I'm going to put up a list of those that I use in the course of this project. The list will be updated as I move along. 

* [Unofficial BitTorrent Protocol Specification][bittorrent-spec]



[first-rust-project]: https://www.tindandelion.com/rust-text-compression/
[project-github]: https://github.com/tindandelion/rust-text-compression
[bittorrent-spec]: https://wiki.theory.org/BitTorrentSpecification