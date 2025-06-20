---
layout: post
title:  "BitTorrent: Key concepts"
date: 2025-06-13
---

Let's start with an overview of the key concepts of BitTorrent architecture to gain a high-level understanding of what we're dealing with.

# General concepts

[BitTorrent][bit-torrent-wiki] is a protocol for _peer-to-peer file sharing_. The idea behind peer-to-peer file sharing is that there's no single central location where the file is stored. Instead, multiple copies of the same file are located at many nodes (_peers_) in the network, and each peer can serve the contents of that file to others. Of course, it all starts with just a single node having a single copy of the file: the _seed_. But as more peers download and host that file, they can also start functioning as seeds themselves.

In addition, the file being distributed is divided into segments called _pieces_. Peers exchange and store pieces between each other, so even if a peer doesn't have the complete file yet, it can act as a source for the pieces it has already downloaded, thus actively participating in the data exchange.

This scheme offers several advantages:

* _Increased download speed:_ peers usually download pieces of files from multiple locations in parallel.
* _Increased availability:_ as the file spreads across the network, more and more peers can seed that file.
* _More balanced bandwidth usage:_ since file pieces are downloaded from multiple locations, this relieves the original seed from having to serve them to everyone single-handedly.

# Torrent file

Information about distributed files is published in the form of [_torrent files_][torrent-file-wiki]. A torrent file contains important metadata about the published file, such as:

* The URL of a _torrent tracker_ (announce URL) that clients initially use to discover peers.
* Information about pieces: the size of a piece in bytes, and a cryptographic hash of each individual piece (SHA-1 or SHA-256).

# Torrent trackers

There is one centralized node in the whole architecture, called a [_torrent tracker_][torrent-tracker-wiki]. Its main purpose is to assist in peer discovery. When a client initiates a file download, it connects to the tracker using the _announce URL_ from the torrent file and obtains a list of peers that can serve the requested file. After that, peer-to-peer communication can continue without a connection to the tracker. Most clients, however, still communicate with the tracker periodically to provide network performance and download statistics.

Since the necessity for a central torrent tracker contradicts the decentralized nature of peer-to-peer communication, a few other methods have emerged to avoid this bottleneck: [_distributed hash tables_][dht-wiki] (DHT) and the [_peer exchange protocol_][pex-wiki] allow for peer discovery without dedicated torrent trackers.


[bit-torrent-wiki]: https://en.wikipedia.org/wiki/BitTorrent
[torrent-file-wiki]: https://en.wikipedia.org/wiki/Torrent_file
[torrent-tracker-wiki]: https://en.wikipedia.org/wiki/BitTorrent_tracker
[dht-wiki]: https://en.wikipedia.org/wiki/Distributed_hash_table
[pex-wiki]: https://en.wikipedia.org/wiki/Peer_exchange




