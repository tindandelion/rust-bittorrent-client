# BitTorrent Client in Rust

![BitTorrent Client](doc/main.gif 'Demo of the TUI')

As I [continue my programming adventures in Rust][first-rust-project], I've decided to launch yet another learning project. This time, it's going to be an implementation of a **simple BitTorrent client**.

### Motivations

Why BitTorrent? One reason is that I've always been interested in how peer-to-peer systems work. There's something very intriguing about how multiple actors can collaborate to accomplish a task without the need for centralized control. By diving deeper into the implementation of such a system, I hope to gain a better understanding of how they work in general.

Another reason is that I expect a project like this will help me deepen my experience with Rust. In particular, I would like to become more familiar with these areas that I haven't explored yet:

- Network programming
- Multi-threaded programming
- Asynchronous programming in Rust
- Programming console UIs

### Project scope

Writing a fully-fledged BitTorrent client is quite a big task, so for my pet project I'd like to scale it down to the essentials. I will consider the project accomplished when my solution is able to do the following:

- ✅ Connect to the torrent tracker to fetch the initial information about the file to download
- ⬜ Download the file from multiple peers in parallel
- ⬜ Serve requests from other peers while the download is ongoing
- ✅ Show the download progress in some form of text-based UI

### Project blog

Like in my [previous project][first-rust-project], I've decided to document my journey in a form of a [project diary][project-blog], to share what I've learned along the way.

[first-rust-project]: https://www.tindandelion.com/rust-text-compression/
[project-blog]: https://www.tindandelion.com/rust-bittorrent-client/
