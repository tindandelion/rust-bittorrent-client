---
layout: post
title:  "Shaking hands with peers"
date: 2025-07-14
---

Our [last significant achievement][prev-post] was getting the list of peer IP addresses and ports from the torrent tracker. This is where the tracker's job ends, essentially. From here on, all communication happens directly between peers via TCP protocol. Our first task in this peer-to-peer exchange is to connect to peers and perform the initial handshake. 

[*Version 0.0.5 on GitHub*][version-0.0.5]{: .no-github-icon}

# Working with TCP in Rust 

First and foremost, we should be able to communicate with the remote host via TCP channel. Rust's standard library provides a struct to handle TCP communication, [`TcpStream`][rust-doc-tcp-stream]. Two constructor methods, `TcpStream::connect()` and `TcpStream::connect_timeout()`, let us establish the client connection with a remote server. Once connected, we can transmit the data over the TCP channel by reading from and writing to the instance of `TcpStream`. 

#### Read and Write traits 

Like many other programming languages, Rust provides abstractions for binary data streams. The value of those abstractions is that it decouples the client code from the implementation of the underlying data transmission mechanism. The same client code can work with a file, or with an in-memory buffer, or with a TCP stream in our case, by means of well-defined abstract interfaces. 

In Rust, these interfaces are represented by [`Read`][rust-doc-read-trait] and [`Write`][rust-doc-write-trait] traits from the `std::io` module. 

Both traits treat data as raw byte sequences. The only required method for the `Read` trait, `read(&mut self, buf: &mut [u8])`, should read the data from the underlying source and put it into `buf`. There are also a few convenience methods, such as `read_to_end()` and `read_exact()`, with provided default implementations. 

Similarly, the `Write` trait requires implementing the method `write(&mut self, buf: &[u8])` to write the sequence of bytes to the destination, and `flush()` to ensure any buffered data reaches its destination. Both methods return `io::Result<()>`.

`TcpStream` struct implements both these traits, and we'll be using their methods to exchange data with peers. 

# BitTorrent handshake 

Now that we know how to send and receive data over the TCP channel, let's dive into the specifics of the BitTorrent protocol. 

The [BitTorrent specification][bit-torrent-spec-handshake] tells us that once the TCP connection is established, the peers must exchange handshake messages. Our client must immediately send the handshake message to the remote peer, and the other side is expected to respond with their handshake message. If the response doesn't match the expected protocol or info hash, the connection should be dropped.

The format for the handshake message goes as follows: 

![Handshake message format]({{ site.baseurl }}/assets/images/handshake-with-peers/handshake-message.svg)

It's a fixed-sized message of 68 bytes in total, where the first 28 bytes are the predefined header, followed by the torrent's info hash, and the peer id. It is rather easy to construct this message as a simple byte array, but I'd like to dive a bit deeper and explore how we can work with structured memory in Rust. I think we're going to need this knowledge when we start working with more complicated message structures.

# Working with structured memory

When doing low-level programming, it's quite common that we need to work with chunks of memory that have some specific layout, just like our handshake message above. If we were writing a program in C, the most convenient way to work with that buffer would be to define a struct type that defines the memory layout, and then use that type in the program to access specific parts as struct's members. This is possible because in C we have total control over how the structs are laid out in memory (with some specific compiler instructions to avoid data padding). 

For example, the type definition of the handshake message in C could look like: 

```c
struct HandshakeMessage {
    unsigned char pstrlen;
    unsigned char pstr[19];
    unsigned char reserved[8];
    unsigned char info_hash[20];
    unsigned char peer_id[20];
}__attribute__((packed));
```

Once we have a structure type defined, it becomes easy to allocate memory for the message buffer as a single piece and access individual parts as struct's members. Also, in C we can treat the pointer to that variable as `unsigned char*` type, and work with that memory as a raw byte array. For example, the hypothetical code for sending and receiving data over TCP channel would look like this: 

```c
struct HandshakeMessage incoming, outgoing; 

tcp_send((unsigned char*)&outgoing, sizeof(outgoing));
tcp_receive((unsigned char*)&incoming, sizeof(incoming));
```

Of course, with that great flexibility come a lot of potential problems. As a low-level language, C allows you to do almost anything with memory pointers, but it's up to you as a programmer to _do it right_. The language itself gives you very little guarantee that the memory is allocated and freed correctly, that you don't read or write to invalid memory regions, etc. In that regard, programming in C is like walking through a minefield. 

# Structured memory buffers in Rust 

Rust has much stricter rules for working with structures in memory, specifically to provide you with some guardrails to avoid common memory-related programming errors. However, sometimes you need to bypass these rules and work with memory directly. Let's see how we can implement the same functionality in Rust. 

#### _HandshakeMessage_ struct 

Just like in C code, the first step is to define a struct type to define the memory layout: 

```rust
const PROTOCOL_ID: &[u8; 19] = b"BitTorrent protocol";

#[repr(C, packed)]
struct HandshakeMessage {
    pstrlen: u8,
    pstr: [u8; PROTOCOL_ID.len()],
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}
```

Notice the `#[repr(C, packed)]` directive at the top of the struct definition. This directive tells the compiler that there are some special rules about the in-memory representation of this type: 

* `repr(C)` tells the compiler to lay out the values of this type exactly as they would be laid out in C. In particular, the order of the fields is preserved. Without this directive, the compiler could reorder fields internally, to optimize the memory footprint; 
* `repr(packed)` instructs the compiler to skip any additional padding between the fields, which it could otherwise insert to optimize memory access. 

#### Raw pointers in Rust 

The second part of the puzzle is to be able to treat references to `HandshakeMessage` values differently for different purposes. When initializing the message buffer, we want to work with it as with a regular struct, but when we send or receive data, we need to treat it as a raw byte array. Normally, Rust won't let us do that: its strict type system and memory safety guarantees prohibit casting between non-primitive types. To be able to do that, we need to move to the realm of _unsafe Rust_. 

Though unsafe Rust sounds dangerous, it's not that scary after all. Among other things, `unsafe` code blocks in Rust allow us to dereference _raw pointers_. Let's see how it works in the case of sending the instance of `HandshakeMessage` using `TcpStream`: 

```rust
impl HandshakeMessage { 
    fn send(&self, dst: &mut impl Write) -> io::Result<()> {
        let buffer_ptr = self as *const Self as *const [u8; size_of::<Self>()];
        unsafe { dst.write_all(&*buffer_ptr) }
    }
}
```

Two things happen here. First, we acquire a raw pointer to `self`, and cast it to the pointer to an array type. That code line is a bit hairy, so let's walk through it piece by piece. First, we create an _immutable raw pointer_ from the reference to `self`: `self as *const Self`. Next, we cast it to the array type: `as *const [u8; size_of::<Self>()]`. The result type is the immutable raw pointer `*const [u8, 68]`. 

The second step is to use that raw pointer by _dereferencing_ it. This code is unsafe, so it needs to be enclosed in an `unsafe` block. This line does it:  

```rust
unsafe { dst.write_all(&*buffer_ptr) }
```

We dereference the raw pointer using the `*` operator, which results in `[u8, 68]` array. Finally, we pass a reference to this array to the `write()` method using the `&` operator. Combined together, these two operators look a bit cryptic: `&*buffer_ptr`. 

In fact, I could combine creating a raw pointer and dereferencing it into a single line of code: 

```rust
let buffer_ptr = unsafe { &*(self as *const Self as *const [u8; size_of::<Self>()]) };
```

However, I decided to split it up as it looked rather illegible to me. 

To receive a message from TCP stream, we do a similar operation, but with a _mutable_ raw pointer `*mut HandshakeMessage`: 

```rust
impl HandshakeMessage {
    fn receive(src: &mut impl Read) -> io::Result<Self> {
        let mut instance = Self::default();
        let buffer_ptr = &mut instance as *mut Self as *mut [u8; size_of::<Self>()];
        unsafe { src.read_exact(&mut *buffer_ptr)? };
        Ok(instance)
    }
}
```

Here, we create a default instance of `HandshakeMessage`, obtain a mutable raw pointer to it, and then dereference that pointer in the `unsafe` block and pass it to `TcpStream::read_exact()` method as a mutable reference to the `[u8, 68]` array. 

It should be noted that there are Rust crates that provide similar functionality while hiding the hairy details of working with raw pointers and unsafe code blocks. One such crate is [`zerocopy`][zerocopy]. However, I decided not to use it yet, to get a feel of working with unsafe Rust. I might switch to using that trait later, though. 

# Putting it all together: probing the peers

Having gotten familiar with those low-level details, I moved on to putting this new knowledge into practice. Let's see how many peers would respond to the handshake with something sensible, by implementing a simple probing routine: 
1. Connect to the peer
2. Send the handshake message and expect to receive a similar message back
3. If successful, print the received peer id
4. On failure, print the error details

To have a place to put the relevant code, I created a new struct called [`FileDownloader`][file-downloader-0.0.5]. As the name implies, I intend it to contain all the logic related to file downloads. But for starters, it has only one method called [`handshake`][file-downloader-handshake-0.0.5]: 

```rust
pub fn handshake(
    &mut self,
    info_hash: Sha1,
    peer_id: PeerId,
) -> Result<String, Box<dyn Error>> {
    HandshakeMessage::new(info_hash, peer_id).send(&mut self.stream)?;
    let response = HandshakeMessage::receive(&mut self.stream)?;
    Ok(String::from_utf8_lossy(&response.peer_id).to_string())
}
```

I also updated the [`main`][main-0.0.5] routine to iterate over the received peer list and execute the probing routine as described above. For now, we do all the work synchronously, one peer entry at a time. To avoid hanging for a long time in case of unresponsive peers, I chose to use relatively small connection and read timeouts: 5 and 10 seconds, respectively. 

Running the new `main` routine, after a while we get the following output on the console: 

```console
[main] $ cargo run

Your announce url is: http://bttracker.debian.org:6969/announce
Total 50 peers
Probing peers...
218.35.173.239:6952     -> OK("A2-1-36-0-.J\t")
188.165.230.19:33721    -> Err(connection timed out)
83.148.245.186:51414    -> OK("-TR4040-3sc8805nhg9f")
71.161.110.91:60000     -> OK("-UT2210-BbQ\u{311}\u{e}\\")
89.187.180.41:56163     -> Err(connection timed out)
68.235.46.165:56833     -> Err(connection timed out)
217.155.7.69:51765      -> Err(Connection refused (os error 61))
149.88.27.212:6881      -> Err(connection timed out)
193.32.127.222:51765    -> Err(connection timed out)
147.30.84.141:26163     -> Err(connection timed out)
77.33.175.70:51413      -> Err(connection timed out)
73.196.29.145:51413     -> Err(Connection refused (os error 61))
66.56.80.113:36787      -> OK("-TR4050-bfasldzz0v6b")
37.194.168.90:59627     -> Err(failed to fill whole buffer)
176.96.240.165:51413    -> OK("-TR3000-ycqp82iba0oa")
188.213.90.144:6996     -> Err(connection timed out)
93.159.191.68:51413     -> OK("-TR3000-giudwpwfbecy")
84.129.153.75:51413     -> OK("-TR4060-d0gfejjwda3q")
72.14.148.3:63853       -> Err(connection timed out)
185.148.3.184:21056     -> OK("-lt0D80-\u{3}$O\n\t")
62.3.58.137:43434       -> Err(connection timed out)
115.45.201.66:51412     -> OK("-TR3000-y4hvnzudz79e")
102.129.252.107:51413   -> Err(Connection refused (os error 61))
219.145.33.116:19602    -> Err(connection timed out)
193.32.127.219:51413    -> Err(connection timed out)
194.126.165.238:51413   -> Err(connection timed out)
146.70.226.233:34139    -> Err(failed to fill whole buffer)
103.229.153.197:51413   -> OK("-TR4050-0g11w0msk6wm")
89.244.85.31:59595      -> Err(connection timed out)
70.120.99.238:5446      -> Err(failed to fill whole buffer)
136.37.73.176:51413     -> OK("-TR4060-975ifdijhkep")
212.104.214.21:58812    -> Err(failed to fill whole buffer)
91.3.121.164:58812      -> Err(Connection refused (os error 61))
212.20.112.112:54121    -> OK("-TR4060-t5btv9gjeodg")
76.69.43.3:6881         -> Err(failed to fill whole buffer)
217.138.252.123:15981   -> Err(failed to fill whole buffer)
94.110.96.137:16881     -> Err(connection timed out)
82.66.165.43:51413      -> Err(connection timed out)
109.201.152.172:1       -> Err(Connection refused (os error 61))
95.153.31.120:39044     -> Err(failed to fill whole buffer)
90.105.114.47:51413     -> Err(connection timed out)
41.254.92.247:30839     -> Err(connection timed out)
91.169.106.182:40000    -> OK("-TR4060-84gjojhbup43")
87.147.66.48:51413      -> Err(Connection reset by peer (os error 54))
202.46.68.230:51413     -> Err(connection timed out)
45.145.110.144:6582     -> Err(connection timed out)
45.131.193.34:6881      -> Err(connection timed out)
87.249.134.6:51413      -> Err(connection timed out)
193.33.56.135:51413     -> Err(Connection refused (os error 61))
185.156.174.178:6881    -> Err(connection timed out)
[main] $ 
```

As we can see, there are a lot of failures, for various reasons: connection timeouts, connection resets, etc. But still, for some peers the handshake results in success, and we get the peer id back. At the bottom line, we got 13 successful responses from the total of 50 peers. I call it a success!

For those curious, a peer id starting with `"-TR"` means that there's [Transmission][transmission] BitTorrent application on the other side. 

# Next steps 

So, the main achievement of this section is that we managed to connect to the peers and exchange handshake messages. From here, I see a few possible next steps: 

1. Continue working on the peer communication. After the initial handshake, we should be able to start downloading the file content. 
2. We are now enumerating peers synchronously, which is quite time-consuming, provided that most peers don't even bother to accept the connection. A better approach would be to try to connect to multiple peers in parallel. That would lead us to the land of asynchronous Rust, which is still the unknown terrain for me.
3. Our handshake handling is very simplistic now. We should make it more robust: at the very least, I need to check that the protocol version and the info hash returned by the peer match what we requested. Ultimately, I'd like to write a test suite to cover various handshake scenarios. That's also an interesting task to tackle. 

I'm not sure yet which task to pick up next; that's yet to be decided. 

[prev-post]: {{site.baseurl}}/{% post_url 2025-06-25-parsing-the-peer-list %}
[rust-doc-tcp-stream]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
[rust-doc-read-trait]: https://doc.rust-lang.org/std/io/trait.Read.html
[rust-doc-write-trait]: https://doc.rust-lang.org/std/io/trait.Write.html
[bit-torrent-spec-handshake]: https://wiki.theory.org/BitTorrentSpecification#Handshake
[zerocopy]: https://docs.rs/zerocopy/latest/zerocopy/
[file-downloader-0.0.5]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.5/src/downloader.rs#L10
[file-downloader-handshake-0.0.5]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.5/src/downloader.rs#L60
[main-0.0.5]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.5/src/bin/main.rs
[transmission]: https://transmissionbt.com/
[version-0.0.5]: https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.5