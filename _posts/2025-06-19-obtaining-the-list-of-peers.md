---
layout: post

date: 2025-06-20
---

In the [last chunk of work][prev-post], I achieved a milestone: making a request to the torrent tracker and getting back a meaningful response. The response is "torrent not found," though, because I'm still passing a fake torrent hash in the request parameters. Today, I want to fix this situation and make the program use the real hash value that it will obtain from the torrent file.

[*Version 0.0.3 on GitHub*][version-0.0.3]{: .no-github-icon}

# What needs to be done

The [BitTorrent specification][bittorrent-spec] describes what has to go into the `info_hash` parameter of the announce request:

> info_hash: urlencoded 20-byte SHA1 hash of the <b>value</b> of the <b>info</b> key from the Metainfo file. Note that the value will be a bencoded dictionary, given the definition of the info key above.

So it sounds like we need to do some additional work when decoding the torrent file: for dictionary fields (which `info` is), we need to calculate and store the SHA-1 hash of their bencoded representation.

Once we have that done, in our `main()` program we'll use the hash value of the `info` key and pass it to the `make_announce_request()` function, instead of a fake one. I'm thinking of something like this:

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

Let's get it done!

# Extending the _Dict_ struct

Recall that we [started to work on the `Dict` struct][dict-struct-first-iteration] a while ago. Back then, I decided to keep things very simple and only add functionality to store and access `ByteString` values. Now it seems that we need to add more power to this struct and support a new method:

```rust
impl Dict {
    pub fn get_dict_sha1(&self, key: &str) -> Option<&Sha1> {
        // Yet to be implemented
    }
}
```

Notice that I still keep things simple, almost to a fault. That `get_dict_sha1()` method looks a bit awkward, and rightly so: it looks too specific for a pretty generic data type like `Dict`. However, I still don't have a clear idea of what the fully-fledged `Dict` public interface should look like, so I resort to the simplest thing that could possibly work in order to achieve the goal. Later on, as our `Dict` struct keeps growing in functionality, I'm going to revisit the interface and get rid of overly specific methods.

Also notice the result type, `Sha1`. I'll get back to that [later in this post](#sha1-struct-the-newtype-pattern).

#### Extending the internal representation

We need to extend the `Dict` struct in two ways:

* First, each `Dict` should also store the SHA-1 value of its encoded content. That's a trivial change: I just added a new field `sha1` to the [`Dict` struct][dict-0.0.3] and updated the constructor `Dict::new()` to initialize this field from the argument.
* Second, its internal `HashMap` should be able to store _both_ `ByteString` and `Dict` values now. That requires a bigger change. Rust has a special kind of data type to represent heterogeneous data: [_enums_][rust-book-enums]. So I created an enum for possible dictionary values, [`DictValue`][dict-value-0.0.3], and changed `Dict::values` to be of type `HashMap<ByteString, DictValue>`.

The latter change broke the code in a few places, but it was easy to fix. Since until now we had only worked with `ByteString`s in dictionaries, the fixes were to wrap `ByteString` values with `DictValue::ByteString`. I simply followed the compiler errors, and voilà! It compiles and the tests are all green. **I can't stress enough the value of a good test suite when refactoring code.**

With these two new pieces of functionality, the implementation of [`Dict::get_dict_sha1()`][get-dict-sha1-0.0.3] became really simple: just fetch a value from the `HashMap` by a given key, and if it's a `Dict` value, ask it for its SHA-1. Otherwise, if the field is not found or it's not a `Dict`, return `None`. Sheesh, piece of cake.

# Enhancing the _Decoder_

We already have some functionality in `Decoder` related to parsing dictionary fields. Specifically, there's a test that verifies that we skip them:

```rust
#[test]
fn skips_dict_elements() {
    let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
    let mut state = Decoder::new(encoded);
    let decoded_dict = state.decode_dict().unwrap();

    assert_eq!(1, decoded_dict.len());
    assert_eq!(None, decoded_dict.get_string("spam"));
    assert!(state.data.is_empty());
}
```

That's no longer a desired feature; I need a different test to ensure that now we _store and access SHA-1 values of dictionary fields_. Here's the code:

```rust
#[test]
fn extracts_and_stores_dict_value_sha1_hashes() {
    let encoded = "d4:spamd3:fooi1234ee3:cow3:mooe".as_bytes();
    let mut state = Decoder::new(encoded);
    let decoded_dict = state.decode_dict().unwrap();

    assert_eq!(2, decoded_dict.len());
    assert_eq!(
        Some(&Sha1::calculate("d3:fooi1234ee".as_bytes())),
        decoded_dict.get_dict_sha1("spam")
    );
    assert!(state.rest_data.is_empty());
}
```

Supporting the new functionality required a few changes in the `Decoder` struct. First, I added the `raw_data` field to hold a reference to the entire bencoded string. Second, I added the `current_pos` field to track the current position in that string. Each time we process a bencoded element, we update this field to point to the beginning of the yet-unprocessed chunk of data. Finally, the `rest_data` field contains a reference to the beginning of the unprocessed chunk. This field is just a convenience: technically, we can always obtain that reference as `self.raw_data[self.current_pos..]`, but the code looked cleaner to me with that field in place, and there's not much effort to maintain it.

With these changes done, it became easy to add the calculation of the SHA-1 hash to the [`Decoder::decode_dict()`][decode-dict-0.0.3] function: we simply record the `current_pos` at the beginning, then proceed to decode the dictionary, and at the end we take the portion of the bencoded string between the recorded and current values of `current_pos`. That's exactly the piece of content whose SHA-1 we need!

# _Sha1_ struct: the _newtype_ pattern

Initially, I passed SHA-1 values as `Vec<u8>`, but very quickly decided to come up with a specific type to represent them: the [`Sha1` struct][sha1-0.0.3]. This is an example of a [_newtype_ pattern][rust-design-patterns-newtype], sometimes also referred to as the [_TinyType_ pattern][tiny-type-pattern]. I've started to use that pattern quite a lot in my code lately, and I think it brings a few benefits to the code:

* It helps me to avoid silly errors. Consider a function `login(user_name: &str, password: &str)`. It's quite easy to make a simple mistake and pass the parameters in the wrong order. If, instead, we have this function defined as `login(user_name: &UserName, password: &Password)`, the compiler will complain if we accidentally mix up the parameter order.
* It creates a natural place to put domain-specific logic and constraints. For example, if password values were required to be non-empty strings, we could place that check into the `Password::from_str(value: &str)` constructor. Elsewhere in the code, we can rely on `Password` values always being valid and avoid unnecessary checks for non-empty strings in those places.
* It makes function types more understandable. When defining function types, we mostly rely on a function signature to figure out what the function is supposed to do. Having a function type like `fn(&str) -> String` doesn't convey much information about the semantics of the argument and the result of such a function. On the other hand, the type `fn(&UserName) -> Password` makes it much clearer.

In some languages, like Java, creating a newtype wrapper incurs quite a lot of boilerplate code. In contrast, in Rust there's almost no code at all, thanks to the magic of the `#derive` macro. That probably explains why newtypes are much more common in Rust.

In my specific case, creating the `Sha1` newtype gave me the following advantages:

* I now have a natural place to put the logic of calculating the value: `Sha1::calculate()`.
* I can be more specific with the [`AnnounceParams::info_hash`][announce-params-0.0.3] field. Instead of declaring it as an obscure `info_hash: Vec<u8>`, I can more specifically declare it as `info_hash: Sha1`, which is clearer. I plan to do the same trick with the `peer_id` field too, when the time comes.
* If I wish to in the future, I can provide a more user-friendly implementation of the `ToString` trait, showing SHA-1 values as hex strings, as they are commonly displayed. No need to do it right now, though.

The only thing that bothers me about `Sha1` is the place where I put it. It's currently located in the `bencoding` module, but it feels like it's not a very natural place for it. On the other hand, I'd like to keep the `bencoding` module free of dependencies on other modules. It's a conflict; maybe later I'll have a better place for it.

# Make it run!

Enough coding; let's run the program and see what response we get from the torrent tracker now:

```console
[main] $ cargo run
Your announce url is: http://bttracker.debian.org:6969/announce
Tracker response: "d8:intervali900e5:peersld2:ip11:88.18.61.544:porti4666eed2:ip13:85.31.128.1114:porti52664eed2:ip13:95.58.175.2324:porti26163eed2:ip14:83.148.245.1864:porti51414eed2:ip14:15.204.231.2024:porti45548eed2:ip14:93.165.240.1044:porti56439eed2:ip14:193.148.16.2114:porti15981eed2:ip13:104.28.224.824:porti16570eed2:ip15:185.193.157.1874:porti25297eed2:ip14:37.120.185.2084:porti51413eed2:ip13:82.102.23.1394:porti39206eed2:ip14:92.101.157.2504:porti58130eed2:ip13:87.58.176.2384:porti62014eed2:ip13:87.58.176.2384:porti62004eed2:ip14:118.142.44.1464:porti6988eed2:ip10:95.33.0.764:porti22936eed2:ip13:73.196.29.1454:porti51413eed2:ip15:163.172.218.2154:porti31951eed2:ip13:63.210.25.1394:porti6886eed2:ip14:82.165.117.1884:porti1eed2:ip12:98.115.1.2084:porti50413eed2:ip15:109.226.251.1304:porti1230eed2:ip14:103.136.92.2524:porti14948eed2:ip14:193.32.127.2224:porti51765eed2:ip14:45.134.212.1014:porti46296eed2:ip13:82.65.230.1594:porti63812eed2:ip13:87.58.176.2384:porti62017eed2:ip13:189.46.193.814:porti9751eed2:ip14:217.174.206.674:porti51413eed2:ip14:183.107.103.254:porti51413eed2:ip13:81.201.16.2474:porti54694eed2:ip11:78.82.25.834:porti6887eed2:ip14:46.231.240.1874:porti50000eed2:ip12:134.3.183.424:porti58578eed2:ip13:73.81.101.1304:porti51414eed2:ip14:89.142.165.1314:porti51413eed2:ip13:82.24.182.2044:porti44346eed2:ip13:87.99.116.1484:porti51413eed2:ip13:87.58.176.2384:porti62015eed2:ip13:38.162.49.1954:porti6881eed2:ip13:82.64.112.1454:porti25561eed2:ip12:212.7.200.734:porti30151eed2:ip14:37.120.210.2114:porti9099eed2:ip12:37.112.5.2244:porti6881eed2:ip12:50.35.176.534:porti62904eed2:ip14:195.206.105.374:porti57402eed2:ip13:73.235.107.364:porti6881eed2:ip14:187.193.191.434:porti51765eed2:ip14:37.120.198.1724:porti12018eed2:ip14:185.21.216.1694:porti32774eeee"
[main] $
```

Hooray! This time, we have a successful response from the tracker with a bunch of data. Skimming through that bencoded content, we can see that there are IP addresses and ports of the peers, which is exactly what we need to start downloading the file!

# Next steps

So we've received the information about the peers from the torrent tracker, but it's still in bencoded format, and we can't access it easily yet. Our `Decoder` is not powerful enough to decode that structure. I think now is the right time to stop beating around the bush and add more beef to the `Decoder` struct so that we can finally get hold of the peers' IP addresses and ports.

[prev-post]: {{site.baseurl}}/{% post_url 2025-06-16-make-http-request %}
[version-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.3
[bittorrent-spec]: https://wiki.theory.org/BitTorrentSpecification#Tracker_Request_Parameters
[dict-struct-first-iteration]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}#just-enough-parsing
[dict-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L12
[rust-book-enums]: https://doc.rust-lang.org/book/ch06-00-enums.html
[dict-value-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L6
[get-dict-sha1-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/dict.rs#L31
[decode-dict-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/decoder.rs#L31
[sha1-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/bencoding/types/sha1.rs
[rust-design-patterns-newtype]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
[tiny-type-pattern]: https://darrenhobbs.com/2007/04/11/tiny-types/
[announce-params-0.0.3]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.3/src/tracker.rs#L5