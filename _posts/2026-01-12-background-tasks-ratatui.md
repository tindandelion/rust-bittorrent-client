---
layout: post
title:  "Ratatui: working with background tasks"
date: 2026-01-12
---

In the [previous post][prev-post] I explored a structure for a typical interactive Ratatui application. However, a BitTorrent client presents additional challenges: the meat of the application is the download process that happens outside a UI-driven render loop. In this section, I'm laying the groundwork for a terminal UI application that does most of its work in the background. 

# The problem 

A typical interactive application performs most of it's tasks as a response to the user's actions. It can be fully driven by the main application render loop. An application that downloads something from the internet is different, though. Downloading a file can take a long time, during which no use input is required, but the changes in the download status still need to be presented in the UI. This is not specific to the internet downloads, of course: we'll have the same problem with any non-interactive activity that takes a lot of time. 

Along with that, the application need to remain responsive to the user input while the download process is ongoing. We should respond to the user input, for example if the user decides to quit the application by pressing some key combination, or resizes the terminal window, etc. 

Generally speaking, there are two logical execution threads in the application: one is performing the download task, and another is reacting to the user's events. Both of these threads update the UI. 

There are several ways to implement logical execution threads. The most obvious one is of course using the physical threads provided by the operating system. Another approach, which is quite natural for IO-heavy applications is asynchronous programming, which allows us to separate the logical execution threads from the physical threads. 

For now, I'm going to proceed with a simple multi-threaded solution. 

# Threads in Rust 

Traditionally, multi-threaded programming has been a pain for software developers. Using multiple threads in the application presents a whole bunch of challenges for the developer and can be a source of hard to catch errors, when multiple threads try to modify the same data in memory. 

Rust, however, provides us with guardrails to avoid data races. It turns out that Rust's borrow checker is also very helpful in concurrent programming. It simply won't allow you to write the code where multiple threads access shared data haphazardly. Instead, the programmer is forced to use a more disciplined approach that avoids the majority of data race conditions. 

The easiest way is to program where each thread operates in its own local state and there's no shared data at all. Since there's no shared data, there's no possibility for data races. But what if threads still need to communicate with each other? One way to solve this is to use message-based communication, where threads exchange messages with each other via a communication channel. 

Rust's standard library provides an implementation for such communication in [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html) module. "mpcs" is an abbreviation for "**m**ultiple **p**roducers, **s**ingle **s**ubscriber"


![Screen recording]({{ site.baseurl }}/assets/images/background-tasks-ratatui/main-tui.gif)