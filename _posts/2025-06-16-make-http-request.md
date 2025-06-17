---
layout: post
title:  "Make HTTP request to the tracker"
date: 2025-06-17
---

We left off our project at the point where we managed to [parse the torrent file (at least partially) and extract the tracker's announce URL][prev-post] from it. Now, it's time to make use of this URL and write some code to send an HTTP request to the torrent tracker.

[*Version 0.0.2 on GitHub*][version-0.0.2]{: .no-github-icon}

# Making the very first HTTP request in Rust

As a warm-up task, I'm thinking about the following: since we already have the announce URL at hand, let's go ahead and just send a request to the tracker as-is. The outcome of this seemingly useless task will be twofold. First, I'll learn how to send HTTP requests in Rust, which I've never done before. Second, I'm curious to see what the tracker will return in response to such an obviously incorrect request.

#### Using the _reqwest_ library

Making a simple HTTP GET request turned out to be quite straightforward in Rust, thanks to the [`reqwest`][reqwest-lib] library. `reqwest` supports both async and blocking clients. For the time being, I think diving into async Rust isn't justified yet, so I decided to stick to the blocking API to keep things simple:

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);

    let response = reqwest::blocking::get(announce_url)?;
    println!("Response: {:?}", response.text()?);
    Ok(())
}
```

Sure enough, when I run the program, I get the following error printed in the terminal:

```
Your announce url is: http://bttracker.debian.org:6969/announce
Error: reqwest::Error { kind: Request, url: "http://bttracker.debian.org:6969/announce", source: hyper_util::client::legacy::Error(SendRequest, hyper::Error(IncompleteMessage)) }
```

To be honest, this error message is rather cryptic. I was expecting something more meaningful in return: some kind of HTTP error with a more descriptive explanation. To explore in more detail what's going on under the hood, I decided to run the same request from the terminal using `curl`:

```console
[main] $ curl -v  http://bttracker.debian.org:6969/announce
* Host bttracker.debian.org:6969 was resolved.
* IPv6: (none)
* IPv4: 130.239.18.158
*   Trying 130.239.18.158:6969...
* Connected to bttracker.debian.org (130.239.18.158) port 6969
> GET /announce HTTP/1.1
> Host: bttracker.debian.org:6969
> User-Agent: curl/8.7.1
> Accept: */*
>
* Request completely sent off
* Empty reply from server
* Closing connection
curl: (52) Empty reply from server
[main] $
```

That makes things a bit clearer. It seems that the server just closes the connection without sending anything back. That's rather abrupt, in my opinion, but it is what it is.

#### They could have done better

Just to whine a little: I think the torrent tracker could have done a better job handling an error situation like this. I sent a malformed GET request that missed some required parameters. In my opinion, the best course of action for the server in that case would be to respond with HTTP error code 400 (Bad Request) and provide details about which parameter is missing in the response body.

But the Debian torrent tracker decided to be rude and simply closed the connection, leaving me clueless about what went wrong. I'm sure they had their reasons for handling it that way; I just wish they had been a bit more descriptive about what was wrong with the request.

Okay, enough whining: let's try to make things right this time.

# Figuring out the "bare minimum"

The BitTorrent specification provides a full list of [tracker request parameters][wiki-tracker-request-params]. There are quite a few of them, but I have a hunch that not all are strictly required. I would like to focus on getting the bare minimum of parameters that would make the tracker stop closing the connection and respond with something meaningful.

So I continued to play with `curl`, and after a few experiments with the command line, I got to the point where the tracker finally responded with a sensible answer:

```console
[main] $ curl -v "http://bttracker.debian.org:6969/announce?info_hash=%124Vx%9A%BC%DE%F1%23Eg%89%AB%CD%EF%124Vx%9A&peer_id=ABCDEFGHIJKLMNOPQRST"
* Host bttracker.debian.org:6969 was resolved.
* IPv6: (none)
* IPv4: 130.239.18.158
*   Trying 130.239.18.158:6969...
* Connected to bttracker.debian.org (130.239.18.158) port 6969
> GET /announce?info_hash=%124Vx%9A%BC%DE%F1%23Eg%89%AB%CD%EF%124Vx%9A&peer_id=ABCDEFGHIJKLMNOPQRST HTTP/1.1
> Host: bttracker.debian.org:6969
> User-Agent: curl/8.7.1
> Accept: */*
>
* Request completely sent off
< HTTP/1.1 200 OK
< Server: mimosa
< Connection: Close
< Content-Length: 39
< Content-Type: text/plain
<
* Closing connection
d14:failure reason17:torrent not founde%
[main] $
```

That's already something! As it turned out, we need _at least two parameters_ to be passed in the URL: `info_hash` and `peer_id`. Both parameters must be exactly 20 bytes long and URL-encoded, exactly as described in the [specification][wiki-tracker-request-params]. Let's put our findings into code!

# Putting it all into code

I've created a new module [`tracker`][tracker-0.0.2] to keep all the code related to communication with the torrent tracker. At the moment, there's not a lot of it — only what we've learned so far.

There's a simple struct [`AnnounceParams`][announce-params-0.0.2] that currently has only two fields: `info_hash` and `peer_id`:

```rust
pub struct AnnounceParams {
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}
```

Choosing `Vec<u8>` as the type for these fields works for now, but it could be a bad choice in the long run. The BitTorrent specification and my previous experiments tell us that both `info_hash` and `peer_id` _must be exactly 20 bytes long_, but vectors can be of arbitrary length. This can be a source of errors in the future: one can easily pass a shorter or longer vector as the hash or the peer ID! I believe the [newtype][new-type-idiom] idiom could help us prevent such errors.

The public function [`make_announce_request()`][make-announce-request-0.0.2] implements all that we've learned so far: it constructs the announce URL with `info_hash` and `peer_id` parameters, sends the request to the tracker, and simply returns the tracker response as a string.

#### Announce URL with parameters: the trouble with vectors

Yet another third-party library, aptly named [`url`][rust-url], helped me construct the full announce URL. In particular, `Url::parse_with_params()` seems to do exactly what I need. There was an obstacle to its use, however, that took me a while to work around. You see, `parse_with_params()` expects that parameter values will be string slices, but in my case, I need to pass binary vectors `info_hash` and `peer_id`! There's no way I can convert them into strings without losing data.

I tried a different approach. There's one more library, [`percent_encoding`][percent-encoding], that I tried to use to encode those binary vectors before passing them to `parse_with_params()`. Unfortunately, it didn't work out: `parse_with_params()` performed one more round of encoding on already encoded strings!

Eventually, I arrived at a solution using the **unsafe** method [`String::from_utf8_unchecked()`][string-from-utf-8-unchecked]. This method allows you to construct strings from byte vectors, bypassing the UTF-8 validity checks. Normally, that can result in a string that contains invalid data (hence the `unsafe` keyword), which may cause problems when using it elsewhere. However, it seemed to work perfectly fine with `Url::parse_with_params()`! It looks like the `url` library simply converts input strings back into byte slices under the hood, and then performs URL encoding on them. I wish there was a way to pass `&[u8]` directly instead, but alas!

To be honest, it still feels hacky to use `String::from_utf8_unchecked()` for this purpose, but currently it looks like the best solution. At any rate, I have a [test][make-announce-url-test] for that functionality, so if it starts to break in the future, I'm prepared to detect it early on.

# What we've got so far

Finally, I updated the [`main()`][main-0.0.2] function to invoke our new functionality:

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents.get_string("announce").unwrap();
    println!("\nYour announce url is: {}", announce_url);

    let announce_params = AnnounceParams {
        info_hash: vec![42; 20],
        peer_id: vec![0x00; 20],
    };
    let response = make_announce_request(announce_url, &announce_params)?;
    println!("Tracker response: {:?}", response);
    Ok(())
}
```
We're still passing fake `info_hash` and `peer_id`, but at least now we make an actual request to the torrent tracker:

```
Your announce url is: http://bttracker.debian.org:6969/announce
Tracker response: "d14:failure reason17:torrent not founde"
```

# Next steps

I've got a couple of immediate tasks on my mind. First, the message that we got from the torrent tracker is actually an error message. It should become an error in the program, not just a text on the screen. Second, we should start passing valid `info_hash` and `peer_id` to the tracker, and finally get something tangible in return.

[prev-post]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}
[version-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.2
[reqwest-lib]: https://docs.rs/reqwest/latest/reqwest
[wiki-tracker-request-params]: https://wiki.theory.org/BitTorrentSpecification#Tracker_Request_Parameters
[tracker-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.2/src/tracker.rs
[announce-params-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.2/src/tracker.rs#L5
[new-type-idiom]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
[rust-url]: https://docs.rs/url/latest/url/
[make-announce-url-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.2/src/tracker.rs#L19
[percent-encoding]: https://docs.rs/crate/percent-encoding/latest
[string-from-utf-8-unchecked]: https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_unchecked
[make-announce-url-test]: https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_unchecked
[make-announce-request-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.2/src/tracker.rs#L10
[main-0.0.2]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.2/src/bin/main.rs#L5






