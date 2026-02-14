---
layout: post
title:  "First integration test"
date: 2026-02-14 
---

I'm about to start doing some serious changes to the core logic of the download process. However, before diving in head-first, I would like to strengthen my test suite by introducing the first _integration test_ that would utilize a real BitTorrent client as a remote peer. Moreover, I would like to create a _controlled test environment_, so that test execution will not rely on anything that lives somewhere else on the internet and therefore is outside my reach. 

# The need for the integration tests 

Until now, I was mainly relying on the suite of unit-level tests to drive the development forward. That served me pretty well so far, but now I'm at the point where I feel that having only unit tests is not sufficient. 

One reason is that at this point I already have a working solution, and I would like to keep it functioning as I introduce more features related to the core of the BitTorrent client. I expect that there will be quite a bit of refactoring of the application structure, so having a high-level test that's decoupled from the internals is going to be helpful when it comes to making serious design changes. 

The second reason is that I'm about to start playing with some pretty low-level stuff, such as (spoiler alert!) non-blocking and asynchronous I/O, and I suspect that some aspects will be hard to test using only unit tests. So far I was testing the application end-to-end by just launching the main application, but it's quite tedious and time-consuming to do so as often as I would like to. 

Finally, as I'm moving towards unknown grounds, I would like to have a local environment with a _real_ BitTorrent client that I could use to write and quickly check any experimental code against it, instead of relying on some kind of a mocked solution. Mocks are useful when you know the details of the behaviour of a mocked part, but they are useless when you're just discovering that behavior. 

Hopefully, I've persuaded myself and the readers that spending some time on a repeatable integration test in a controlled test environment is a good investment of time and effort, so let's move on. 

# Step one: local BitTorrent client 

In fact, I've already peeked into using a local BitTorrent client installation when I was [experimenting with improving the download speed][link-missing]. Let's use that experience to start elaborating our first integration test! 

#### Test torrent file 

In theory, I could still use the Debian torrent file for integration tests, but it's big size makes it cumbersome. Even in the local environment it takes more than 10 seconds to download, and it's a drag to wait for so long. Luckily, it's pretty easy to create a torrent from a smaller file of my own choice, and use it in the integration tests. I've decided to use the full text of _War and Peace_ by Leo Tolstoy, [freely available](https://www.gutenberg.org/cache/epub/2600/pg2600.txt) on the Internet. With a decent size of 3.3 megabytes, it looks like a good pick: not too short and not too big. 

![Local Transmission]({{ site.baseurl }}/assets/images/first-integration-test/transmission.png)


Having added this file as a torrent to my local Transmission, I can now write my first integration test: 

```rust 
const DATA_FILE_PATH: &str = "test-env/war-and-peace/war-and-peace.txt";
const TORRENT_FILE_PATH: &str = "test-env/war-and-peace/war-and-peace.torrent";
const LOCAL_IP_ADDRESS: &str = "127.0.0.1:54196";

#[test]
fn download_war_and_peace() -> Result<()> {
    let peer_address = LOCAL_IP_ADDRESS.parse()?;
    let torrent = Torrent::read_file(TORRENT_FILE_PATH)?;
    let peer_id = PeerId::default();

    let (tx, _rx) = mpsc::channel();
    let downloaded = torrent.download_from(vec![peer_address], peer_id, &tx)?;

    assert_eq!(read_test_file()?, downloaded.content);
    Ok(())
}

fn read_test_file() -> Result<Vec<u8>> {
    let content = fs::read(DATA_FILE_PATH)?;
    Ok(content)
}
```

Run the test, and it passes. The test takes 3 seconds to execute, I can tolerate that: 

```console
[main] $ cargo test -q --test wap_download

running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
```

So now we have an integration test that works in the local environment. But I'm not fully satisfied yet: I don't want to rely on a manually configured Transmission client. I think we can do even better: let's make a fully automated test environment that does not involve a manual setup! 

