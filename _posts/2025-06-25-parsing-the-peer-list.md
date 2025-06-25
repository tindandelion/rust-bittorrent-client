---
layout: post
title:  "Parsing the peer list from tracker"
date: 2025-06-25
---

[Last time][prev-post], we managed to fetch the list of peers from the torrent tracker for our [sample torrent file][torrent-file-0.0.4]. I left off by simply dumping the response from the torrent tracker onto the screen, and now I would like to pick up on that and actually parse the tracker's response so that we can get our hands on peers' IP addresses and ports. That's going to be our next step towards making connections to peers. 

# Tracker response structure 

Looking at the [description of tracker response][wiki-tracker-response], we can see that it's a bencoded dictionary with a few fields. For us, the most interesting one is the `peers` field that contains a list of dictionaries, where each entry includes information about an individual peer: 

* `ip`: peer's IP address (string); 
* `port`: peer's port number (integer). 

Indeed, we can see these values in the [raw tracker response][raw-tracker-response]. The specification also mentions the field `peer id`, but skimming through the response string, I don't see any traces of that field inside. I assume that field is optional. In any case, `ip` and `port` are the most important for us now. 

# A need for a more powerful decoder 

Now, that's a fairly complex structure: a dictionary that contains a field that's a list of dictionaries. Recall that in our [previous work][handicapped-decoder] we implemented _some_ decoding of bencoded values, but the functionality in this area is still quite basic. In particular, we have no way of parsing nested complex structures yet. 

Moreover, I've been adding accessor methods to the `Dict` struct in a somewhat haphazard manner, guided by what data I needed at the moment, which resulted in `Dict` having a few bespoke methods, such as `get_string()` and `get_dict_sha1()`. If I continue in that manner, I risk polluting the `Dict` interface with more methods of that kind. That's not a good design. 

It looks to me that we've come to the point where we need to pt more effort into parsing the bencoded data: 

* Our `Decoder` must be able to handle complex nested data structures, such as dictionaries containing lists of dictionaries; 
* We need a more coherent data model to represent the decoded data, so that we can work with it through a relatively narrow interface. 

# Decoder implementation 

Here I'll describe my approach to representing bencoded data structures in the code. I think I managed to come up with a more or less robust implementation that's also relatively easy to use. There are probably more idiomatic ways to implement these concepts in Rust, but I'm clueless of them at my current level. As I learn more, I might revisit this implementation and reshape things. 

#### _BencValue_ enum

At the center of the implementation, there's [BencValue enum][benc-value-0.0.4], that replaces `DictValue` that I [introduced earlier][dict-value-ref]: 

```rust 
pub enum BencValue {
    Int(i64),
    ByteString(ByteString),
    Dict(Dict),
    List(List),
}
```

This enumeration encompasses all four possible data types that we can encounter in bencoded structures: byte strings, integers, dictionaries, and lists. To access the underlying values, we could use pattern matching to destructure `BencValue`s, but for convenience, I also implemented a few methods that do this work for us: `as_int()`, `as_byte_string()`, `as_dict()`, and `as_list()`. Each of these methods returns an `Option`. The idea here is that these methods will return `None` if they're called on a `BencValue` of the wrong type. The alternative could be to return a `Result`, but I think `Option` works just fine for now. 

We [already saw][just-enough-parsing] the `ByteString` struct that I use to represent bencoded strings. As you may remember, we can't use the built-in `String` type because bencoded strings can contain binary data incompatible with UTF-8 encoding. 

The `Dict` struct, which we [also saw before][just-enough-parsing], is a representation of bencoded dictionaries. Essentially, it's a wrapper around `HashMap<ByteString, BencValue>`, but with one important addition: it also carries the `sha1` field that contains the value of the SHA-1 hash of its encoded content. Recall that [we need this value][info-hash-value] specifically for the `info` field from the torrent file to pass it as a parameter to the tracker's announce request. 

I've cleaned up the interface of `Dict`, removing those pesky `get_string()` and `get_dict_sha1()` methods. Now, there's a single method to fetch the value by its key: 

```rust
fn get(key: &str) -> Option<&BencValue>
```

Once the value is fetched, the client code can then use `BencValue::as_*` methods to access the underlying data. Here's, for example, how we can fetch the announce URL from the torrent file content: 

```rust
let announce_url = torrent_file_contents
    .get("announce")
    .and_then(|v| v.as_byte_string())
    .map(|v| v.to_string())
    .expect("Unable to retrieve announce URL");
```

It looks a bit wordy on the client side, but I didn't want to introduce specific methods `get_string()`, `get_dict()`, etc. to the `Dict` interface, especially since similar methods would then need to be introduced to the `List` type. It's a trade-off between the simplicity of the interface and ease of use. I'll keep an eye on it in the future to see if there are any tricks in Rust that I could use to get rid of this excess verbosity. 

Finally, `List` is just a type alias for `Vec<BencValue>`. 

You may notice that `BencValue` is indirectly a recursive data structure: `BencValue::Dict` represents a dictionary of `BencValue`s, and `BencValue::List` does the same for lists. That reflects the recursive structure of the [bencoding format][wiki-bencoding]. 

#### _Decoder_ struct

The [`Decoder`][decoder-0.0.4] struct has also been improved: now it can recursively parse the bencoded content, building the nested structure of `BencValue`s. It comes with quite an extensive test suite that covers various success and failure scenarios. I'm quite confident in its capabilities, but time will show if I did a good job here. 

I've decided to keep `Decoder` private to the module and instead exposed a single top-level function [`decode_dict()`][decode-dict-0.0.4] from the `bencoding` module. Parsing top-level dictionaries has been the only use case we've encountered so far.

# Test-drive: parsing the peer list 

Now, it's time to put our empowered decoding machinery into use and parse the tracker's response into a more convenient data structure: 

```rust
pub struct Peer {
    pub ip: String,
    pub port: u16,
}

pub fn get_peer_list_from_response(tracker_response: &[u8]) -> Result<Vec<Peer>, Box<dyn Error>> {
    let decoded_response = decode_dict(tracker_response)?;

    let peers_list = decoded_response.get("peers").unwrap().as_list().unwrap();
    let x = peers_list
        .iter()
        .map(|peer| peer.as_dict().unwrap())
        .map(|peer| {
            let ip = peer
                .get("ip")
                .and_then(|v| v.as_byte_string())
                .map(|v| v.to_string())
                .unwrap();
            let port = peer
                .get("port")
                .and_then(|v| v.as_int())
                .map(|v| *v as u16)
                .unwrap();
            Peer { ip, port }
        })
        .collect();

    Ok(x)
}
```

For simplicity, I'm just using `unwrap()` here and there; better error handling is yet to come. But the core functionality is pretty sound: we iterate over the `peers` list and covert it into a list of `Peer` structs that carry peers' IP addresses and ports. 

Running the slightly updated [main][main-0.0.4], I now get the following output from the program: 

```console
[main] $ cargo run
Your announce url is: http://bttracker.debian.org:6969/announce
Peer list (total 50 peers):
Top 10 peers:
92.101.180.9:58130
185.209.199.91:51820
84.74.128.132:51413
172.116.246.83:65210
81.225.109.185:51413
136.37.73.176:51413
83.114.68.205:58630
86.115.226.162:49152
145.239.206.200:44444
178.140.191.150:50501
[main] $
```

# What's next? 

So, I managed to get the IP addresses and ports of the peers. In theory, I can go on to start peer-to-peer communication. But I'm a bit confused now about what the best course should be. For one, I'm worried that the code I wrote to communicate with the torrent tracker is not in the best shape now. It's mostly ad-hoc experimental snippets of code that lack proper structure, tests, and error handling — all that constitutes good software in my opinion.

I don't want to rush ahead. Instead, I think I need to do an intermediate reflection session and plan what to do next. 




[prev-post]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}
[torrent-file-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.4/test-data/debian-12.11.0-amd64-netinst.iso.torrent
[wiki-tracker-response]: https://wiki.theory.org/BitTorrentSpecification#Tracker_Response
[wiki-bencoding]: https://wiki.theory.org/BitTorrentSpecification#Bencoding
[raw-tracker-response]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}#make-it-run
[benc-value-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.4/src/bencoding/types/benc_value.rs#L4
[just-enough-parsing]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}#just-enough-parsing
[decoder-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.4/src/bencoding/decoder.rs
[decode-dict-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.4/src/bencoding.rs#L7
[main-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/bin/main.rs#L7
[handicapped-decoder]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}#the-handicapped-decoder
[dict-value-ref]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}#extending-the-internal-representation
[info-hash-value]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}#what-needs-to-be-done