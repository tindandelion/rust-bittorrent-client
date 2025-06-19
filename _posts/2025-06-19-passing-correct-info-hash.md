---
layout: post
title:  "Passing the correct torrent hash"
date: 2025-06-19
---

In the [last chunk of work][prev-post] I achieved a milestone: making a request to the torrent tracker and getting back a meaningful response. The response is "torrent not found", though, because I'm still passing a fake torrent hash in request parameters. Today, I want to fix this situation and make the program use the real hash value that it will obtain from the torrent file. 

[*Version 0.0.3 on GitHub*][version-0.0.3]{: .no-github-icon}

# What needs to be done 

The [BitTorrent specification][bittorrent-spec] specifies what has to go into `info_hash` parameter of the announce request: 

> info_hash: urlencoded 20-byte SHA1 hash of the <b>value</b> of the <b>info</b> key from the Metainfo file. Note that the value will be a bencoded dictionary, given the definition of the info key above.

So it sounds that we need to do some additional work when decoding the torrent file: for dictionaries values, we need to calculate and store the SHA-1 hash of its encoded representation. 

In our `main()` program we'll be using the hash value of `info` key, and pass it to the `make_announce_request()` function, instead of a fake one. I'm thinking about something like that: 

```rust 
fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents
        .get_string("announce")
        .expect("Unable to retrieve announce URL");

    // Get the SHA-1 value of `info` key
    let info_hash = torrent_file_contents
        .get_dict_sha1("info")
        .expect("Unable to retrieve SHA-1 hash of `info` key")
        .clone();
    println!("\nYour announce url is: {}", announce_url);

    // Pass the obtained SHA-1 value to the request
    let announce_params = AnnounceParams {
        info_hash: info_hash,
        peer_id: vec![0x00; 20],
    };
    let response = make_announce_request(announce_url, &announce_params)?;
    println!("Tracker response: {:?}", response);
    Ok(())
}
```

# Extending _Dict_ struct

Recall that we [started to work on the `Dict` struct][dict-struct-first-iteration] a while ago. Back then, I decided to keep things very simple, and only add functionality to store and access `ByteString` values. Now it seems that we need add some more power to this struct, to support a new method: 

```rust 
impl Dict {
    pub fn get_dict_sha1(&self, key: &str) -> Option<&Sha1> {
        // Yet to be implemented
    }
}
```

Notice that I still keep things simple, almost to a fault. That `get_dict_sha1()` method looks a bit awkward, and rightly so: it looks too specific for a pretty generic data type `Dict`. However, I still don't have a clear idea of what the fully-fledged `Dict` public interface should look like, so I resort to the simplest thing that could possibly work in order to achieve the goal. Later on, as our `Dict` struct keeps growing in functionality, I'm going to revisit the interface and get rid of overly specific methods. 

Also notice the result type `Sha1`. I'll get back to that later. 

#### Extending the internal representation 

We need to extend `Dict` struct in two ways: 

* First, each `Dict` should also store the SHA-1 value if its encoded content. That's a trivial change: I just added a new field `sha1` to the [`Dict` struct][dict-0.0.3], and updated the constructor `Dict::new()`. 
* Second, its internal `HashMap` should be able to store _both_ `ByteString` and `Dict` values now. That requires a bigger change. Rust has a special kind of data types to represent heterogeneous data: [_enums_][rust-book-enums]. So I created an enum for possible dictionary values: [`DictValue`][dict-value-0.0.3], and changed the `Dict::values` to be of type `HashMap<ByteString, DictValue>`. That change broke the code in a few places, but the fixes were quite simple. Since by now we only worked with `ByteString`s in dictionaries, the fixes were to wrap `ByteString` values with `DictValue::ByteString`. I simply followed the compiler errors, and voíla! It compiles and the tests are all green. **I can't stress enough the value of a good test suite when refactoring code.** 

With these two new pieces of functionality, the implementation of [`Dict::get_dict_sha1()`][get-dict-sha1-0.0.3] became really simple: just fetch a value from the `HashMap` by a given key, and as soon as it's a `Dict` value, ask it for it's SHA-1. Otherwise, if the field is not found or it's not a `Dict`, return `None`. Sheesh, piece of cake. 

# Enhancing the _Decoder_

TBD

[prev-post]: {{site.baseurl}}/{% post_url 2025-06-16-make-http-request %}
[version-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.3
[bittorrent-spec]: https://wiki.theory.org/BitTorrentSpecification#Tracker_Request_Parameters
[dict-struct-first-iteration]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}#just-enough-parsing
[dict-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L12
[rust-book-enums]: https://doc.rust-lang.org/book/ch06-00-enums.html
[dict-value-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L6
[get-dict-sha1-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L31