---
layout: post
title:  "Better logging with Tracing"
date: 2025-11-29
---

Another thing that has interested me for a while was how to approach logging in a Rust application. Until now, I was just relying on `println()` macro to display significant events in my torrent client. However, this is not a sustainable approach: in real-world applications, you don't want `println()` statements here and there: your approach to logging should be more systematic. 

Usually in applications, developers rely on some sort of a _logging library_ that offers developers a flexible and configurable approach to manage logs. I was unaware of such libraries in Rust, until I came across an excellent video by Jon Gjengset, [Decrusting the tracing crate](https://youtu.be/21rtHinFA40?si=yHWqdFj0j08thUo1), that introduced me to the [Tracing](https://docs.rs/tracing/latest/tracing/) framework and gave me the answer I needed. 

# Introduction to tracing 

# Using tracing in my project 

As described above, Tracing provides a developer with powerful capabilities to keep track of what's going on in the application using spans and events. I can see how it can be useful to record useful information on many levels in my application: 

* On the low level, we can keep track of the message exchange with the remote peer: what messages we send to the peer and what we receive from it; 
* On a higher level, we can record the process of downloading the file piece by piece. Since downloading a single piece is also done in separate blocks, it makes sense to keep track of requesting and receiving individual blocks as well. 

At this time, however, I want to keep things simple. My goal is to get rid of `println()` statements in the code, and replace them with appropriate `info()` and `debug()` macros from Tracing. 


