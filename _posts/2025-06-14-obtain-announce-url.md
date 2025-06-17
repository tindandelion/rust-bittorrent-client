---
layout: post
title:  "First step: Fetching the announce URL"
date: 2025-06-14
---

As mentioned in the [previous post][prev-post], BitTorrent clients start the download by first making a request to the torrent tracker using the announce URL to retrieve the list of available peers. The announce URL is taken from the torrent file. It seems that my very first task should be **to parse the torrent file and extract the announce URL**.

[*Version 0.0.1 on GitHub*][version-0.0.1]{: .no-github-icon}

# Picking a torrent

To get things started, I'm going to need a real torrent file that I'll use throughout the project. There's no shortage of torrent resources on the Internet, but I need to be cautious here. BitTorrent is often used to distribute unlicensed content, and I don't want to accidentally participate in any illegal activity.

Luckily, there are plenty of resources that are legal to distribute via BitTorrent. For example, many Linux distributions offer their official ISO images via BitTorrent. For my purposes, I've decided to use [Debian's `netinst` ISO image][debian-netinst-torrent].

I've downloaded the torrent file and saved it into the [`test-data`][test-data-0.0.1] directory. I guess that's going to be my primary data source throughout the project.

# Torrent file format: _bencoding_

The BitTorrent protocol uses a special data format to organize structured data, called [_bencoding_][bencoding-doc]. Conceptually, I've found it to be similar to JSON, but denser and simpler in terms of supported data types. In most cases, bencoding uses special _delimiter symbols_ to identify the type of the data piece and specify its bounds within the encoded data array.

There are only four data types in bencoding:

* _Byte strings_ (`4:spam`). Most strings are in UTF-8 format, except that sometimes they are not. For example, sometimes string data contains SHA-1 values as raw bytes.
* _Integers_ (`i12345e`). Integers can be negative or positive, of arbitrary length. It is mandatory to handle at least signed 64-bit integers.
* _Lists_ (`l4:spam4:eggse`). Lists can contain any bencoded type, including other lists and dictionaries.
* _Dictionaries_ (`d3:cow3:moo4:spam4:eggse`). Dictionaries contain key-value pairs. Keys are strings; values can be any bencoded type, including lists and other dictionaries.

The torrent file itself is essentially a bencoded dictionary that describes the published file. Skimming through [the specification][torrent-file-spec], we can see that it must contain the `announce` key at the top level, whose value is the announce URL that we need. That's quite handy: I have a hunch that I probably won't need the entire parsing machinery at the very beginning. It seems that to accomplish the task of retrieving the announce URL, I can get away with just a fraction of the full functionality.

# Just enough parsing

Now I'm ready to start writing some code. Since parsing bencoded data is a small, isolated piece of the entire project, I decided to put all related code into the module called [`bencoding`][bencoding-module-0.0.1].

In order to complete the task, I only need a couple of data types to model bencoded data:

* [`ByteString`][byte-string-0.0.1] to represent a byte string. Note that I can't use Rust's `String` type for bencoded strings. As I mentioned earlier, bencoded strings sometimes contain raw byte data, such as SHA-1 values. In contrast, Rust's `String` type can't contain just any bytes: the data must be a valid UTF-8 string. Therefore, I had to come up with a custom type, essentially a wrapper for a `Vec<u8>`. For convenience, this type also defines a method `as_str()` which can be used to convert the underlying data into a `&str` reference, when _we are certain_ that it's a valid UTF-8 string.
* [`Dict`][dict-0.0.1] to represent a dictionary. Here I made a significant simplification for the very first version. At the moment, we assume that `Dict` will contain _only `ByteString` data:_ just enough to fetch the `announce` field. I will add more functionality to support other data types as soon as the application needs it. Its helper method `get_string(key: &str)` lets us retrieve the string value for a given key.

Finally, we have the [`Decoder`][decoder-0.0.1] struct: our workhorse to do the decoding work. For the time being, it has only one public method: `decode_dict()`, just enough to parse the torrent file and retrieve its top-level string fields.

#### The handicapped _Decoder_

As I said [earlier](#just-enough-parsing), my primary focus right now is to extract the `announce` top-level field from the torrent file, so the `Decoder` is only capable of parsing dictionaries with string fields. And it does its job rather well: the functionality for [parsing bencoded strings][decoder-0.0.1-decode-string] is pretty much fleshed out, including handling various error conditions that I could have imagined so far. For other types, however, its functionality is still lacking: it simply skips over fields that are not strings.

Unfortunately (for me), as I started to implement the decoding functionality, I soon realized that I couldn't get away with the simplest possible thing. You see, the bencoding format is recursive in nature: dictionaries can contain other dictionaries, or lists of dictionaries, etc. In order to correctly jump to the next string field in the dictionary, I had to determine its position in the encoded data array, which in turn meant that I had to honestly parse all fields in between: integers, lists, and other dictionaries, even though technically I don't need their values right now.

So I came up with a compromise: implement just enough parsing functionality to be able to skip these fields, and focus only on parsing string fields comprehensively.

#### Why stop there?

You might wonder: why stop? If I have to implement essentially all parsing functionality just to retrieve the string fields, why not go a step further and support all other data types?

The answer is: I don't want to be bothered with it right now. It's one thing to parse just enough to detect the field boundaries within the encoded data and skip the content. But it requires additional work to _parse the value correctly_: at the very least, you'll have to deal with various error conditions, write the tests for it, etc. I just want to put off this work until I really need it.

In addition, I'm not sure yet how to represent different bencoded types in code. It's simple to think that a dictionary contains only string values: the representation of such a structure maps straightforwardly to `HashMap<ByteString, ByteString>`. But when you have to deal with a dictionary of heterogeneous values, in strongly typed languages such as Rust you need to add some complexity. Finally, I have no clear vision yet of what the interface for a `Dict` type of heterogeneous values should look like, to be convenient in use.

With all these considerations and doubts, I think it's more pragmatic to put off the fully-fledged parsing functionality until later, hoping that I'll have a better understanding of the problem once I need it.

# Putting it all to work

At this point, I have some functionality I can demonstrate to the world: I should be able to take my test torrent file, parse it, and access the `announce` field. Let's put it to work!

Here's my very first [`main`][main-0.0.1] function:

```rust
fn main() {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);
}
```

And, once we run it, we indeed see the announce URL in the output:

```
Your announce url is: http://bttracker.debian.org:6969/announce
```

Hooray, we've got our very first feature working! Tagging it as [version 0.0.1][version-0.0.1] in GitHub and moving on to the next task.

# Next steps

Now that I have the announce URL at hand, I can proceed to make an actual request to the torrent tracker. I suspect I'm going to split this work into two parts. For starters, I need to learn how to make HTTP requests in general in Rust. Second, I need to create a proper request to the tracker with all needed parameters, and parse its response.

Looks like there's [a lot of fun ahead][next-post]!


[prev-post]: {{site.baseurl}}/{% post_url 2025-06-13-bittorrent-overview %}
[debian-netinst-torrent]: https://cdimage.debian.org/debian-cd/current/amd64/bt-cd/
[test-data-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.1/test-data
[bencoding-doc]: https://wiki.theory.org/BitTorrentSpecification#Bencoding
[torrent-file-spec]: https://wiki.theory.org/BitTorrentSpecification#Metainfo_File_Structure
[bencoding-module-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.1/src/bencoding
[byte-string-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/bencoding/types.rs#L4
[dict-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.1/src/bencoding/types.rs#L26
[decoder-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.1/src/bencoding/decoder.rs#L8
[decoder-0.0.1-decode-string]: https://github.com/tindandelion/rust-bittorrent-client/blob/main/src/bencoding/decoder.rs#L58
[main-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.1/src/bin/main.rs
[version-0.0.1]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.1
[next-post]: {{site.baseurl}}/{% post_url 2025-06-16-make-http-request %}