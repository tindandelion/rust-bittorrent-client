---
layout: post
title:  "Discovering serde"
date: 2025-11-20
---

At the beginning of this project, I [implemented a simple parser for .torrent files][bencoding-post]. It was an interesting exercise to get familiar with [bencoding](https://wiki.theory.org/BitTorrentSpecification#Bencoding) format. However, there is already an implementation for parsing bencoded data, which comes as an extension to a popular Rust deserialization library called __serde__. I think it is a good opportunity to get familiar with this library and switch to using serde for working with bencoded data. 

# What is __serde__? 

In real-world programming, the need to serialize and deserialize data to and from various text and binary formats appears very often. Reading configuration files, passing data as JSON in API requests, all require us programmers to be able to represent data in various text or binary formats. Along with that, data serialization is probably one of the most tedious and boring tasks for a programmer to work on. No wonder, there's a multitude of libraries in most programming languages, that help programmers to simplify this task. 

Normally, a library for serialization/deserialization helps you convert internal data structures, such as objects or structs, to their serialized representation in a particular format without the need to write code by hand. These libraries can use runtime reflection or other type of meta-information to explore the structure of a data object, and automatically generate the serialized data. 

In Rust, a popular library to help programmers with serialization/deserialization is [serde](https://serde.rs/). Unlike other libraries that rely on reflection, serde uses Rust's trait system and macros to generate the serialization code. Another remarkable feature of serde is that it's not bound to any particular data format. Instead, serde builds its [internal data model](https://serde.rs/data-model.html) that provides a level of abstraction between data types and serialization formats. Support for specific data formats is outsourced to third-party implementations that come in separate crates. This separation makes serde very extensible: to add a support for some new data format, anyone can write an implementation in a separate crate, without the need to make changes to `serde` library itself. 

Also, serde gives developers the mechanisms to plug into the serialization/deserialization process to customize the process for their specific needs. There's a bunch of [attributes](https://serde.rs/attributes.html) out of the box to customize generated serialization code for common scenarios. If they are still not enough, a developer can [write their own](https://serde.rs/custom-serialization.html) fully custom serialization logic. 

# Reading torrent file 

## Defining the data types types

We begin by defining the data types that will hold the parsed torrent data: 

```rust
use serde::Deserialize;
use serde_bytes::ByteBuf;

#[derive(Debug, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub name: String,
    pub length: u64,
    #[serde(rename = "piece length")]       // [1]
    pub piece_length: u64,
    #[serde(with = "serde_bytes")]          // [2]
    pub pieces: Vec<u8>,
}
```

We annotate these data types with `#[derive(Deserialize)]` attribute. That provides us with a sensible implementation of serde's [`Deserialize`] trait. we have to to a few customizations here, though. 

First, the `piece_length` field. In torrent file, the name of this field contains a space: `field length`. Obviously, it can't be used directly as the field name in Rust because of the space character. To work around that, we use #[serde(rename = "piece length")] [field attribute](link) to instruct serde that the field `field length` in serialized data maps to the the field `field_length` in our `Info` struct. 

The second trick is the `pieces` field. In torrent file, it contains concatenated SHA-1 hashes for all file pieces. The problem is, this is binary data. If we had `pieces` defined as a `String`, we'd get a runtime error that we're trying to deserialize an invalid UTF-8 string. Luckily, we can get round this problem with the help of another crate, [serde_bytes](link). This crate provides us with utilities to efficently deserialize raw byte data into a `Vec<u8>`. We plug in this module by using `#[serde(with = "serde_bytes")]` attribue on `pieces` field. 

## Handling deserialization 

Now that we have `Torrent` struct that implements `Deserialize` trait, we can invoke the `Deserialize::deserialize()` method: 

```rust
let mut d: Deserializer<_> = ...;                       // ??? 
let torrent = Torrent::deserialize(&mut d).unwrap();
```

But hold on a second. `deserialize()` method requires an implementation of `Deserializer` trait to be passed in as an argument. Where does that implementation come from? 

This is where we see the separation of responsibilites between serde and specific data format implementations. You see, by itself serde knows nothing about data formats. It works exclusively with the `Deserializer` trait to do its part of the job: provide a link between Rust data type and the deserializer. It is the job of a specific implementor of `Deserializer` tarit to handle pesky details of parsing the data in specific data format. In other words, `Deserializer` trait provides an abstract _architectural boundary_ between `serde` core and the data parser implementation. 

To provide the implementation of the `Deserializer` trait that knows how to parse bencoded data, we need another crate, [`serde_bencode`](link). Having added this crate to project's dependencies, we are now able to read and parse torrent files: 

```rust
let f = File::open(TORRENT_FILE).unwrap();
let mut d = serde_bencode::Deserializer::new(f);
let torrent = Torrent::deserialize(&mut d).unwrap();
```

Or, using a utility function `serde_bencode::from_bytes()`, we can skip the details: 

```rust
let content = std::fs::read(TORRENT_FILE).unwrap();
let torrent: Torrent = serde_bencode::from_bytes(&content).unwrap();
```

Voila! With just a few lines of code, we have a complete implementation of a torrent file parser! 


[bencoding-post]: {{site.baseurl}}/{% post_url 2025-06-14-obtain-announce-url %}
