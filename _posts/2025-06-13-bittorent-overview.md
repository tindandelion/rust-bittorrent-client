---
layout: post
title:  "BitTorrent key concepts"
date: 2025-05-18
---

Let's quickly go through the key concepts of BitTorrent archiecture, in order to gain some high-level understaning understaning of what we're dealing with. 

# General concepts

[BitTorrent][bit-torrent-wiki] is a protocol for _peer-to-peer file sharing_. The idea of peer-to-peer file sharing is that there's no single central location where the file is stored. Instead, multiple copies of the same file are located at many nodes _(peers)_ in the network, and each peer can serve the contents of that file to others. Of course, it all starts with just a single node having a single copy of the file: the _seed_, but as more peers download and host that file, they also can start functioning as its seeds. 

In addition, the file being distributed is divided into segments, called _pieces_. Peers exchange and store pieces between each other, so even if a peer doesn't have the complete file yet, it can act as a source of the pieces it's already downloaded, thus actively participating in the data exchange. 

This scheme allows for the following advantages: 

* _Increased download speed:_ peers usually download pieces of files from multiple locations in parallel;
* _Increased availability:_ as the file spreads across the network, there are more and more peers that can seed that file;
* _More even bandwidth:_ since file pieces are downloaded from multiple locations, this relieves the original seed from having to serve them to everybody single-handedly. 

# Torrent file 

The information about distributed files is published in a form of [_torrent files_][torrent-file-wiki]. A torrent file contains important metadata about the published file, such as: 

* The URL of a _torrent tracker_ (announce URL) that clients initially use to discover peers; 
* The information about pieces: the size of a piece in bytes, and a cryptographic hash of each individual piece (SHA-1 or SHA-256); 

# Torrent trackers 

There is one centralized node in the whole architecture though, called a [_torrent tracker_][torrent-tracker-wiki]. Its main purpose is to act as an assistant in peer discovery. When a client initiates file download, it connects to the tracker by the _announce URL_ from the torrent file, and obtains the list of peers that can serve the requested file. After that, peer-to-peer communication can continue without the connection to the tracker. Most clients, however, still communicate with the tracker periodically, to provide network performance and download statistics. 

Since the necessity for a central torrent tracker contradicts the decentralized nature of peer-to-peer communication, a few other methods have emerged to avoid this bottleneck: [_distributed hash tables_][dht-wiki] (DHT) and [_peer exchange protocol_][pex-wiki] allow for peer discovery without dedicated torrent trackers. 


[bit-torrent-wiki]: https://en.wikipedia.org/wiki/BitTorrent
[torrent-file-wiki]: https://en.wikipedia.org/wiki/Torrent_file
[torrent-tracker-wiki]: https://en.wikipedia.org/wiki/BitTorrent_tracker
[dht-wiki]: https://en.wikipedia.org/wiki/Distributed_hash_table
[pex-wiki]: https://en.wikipedia.org/wiki/Peer_exchange




