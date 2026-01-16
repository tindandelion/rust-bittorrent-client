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

Rust's standard library provides an implementation for such communication in [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html) module. "mpcs" is an abbreviation for "**m**ultiple **p**roducers, **s**ingle **s**ubscriber". The function `channel()` will create an asynchronous communication channel, returning a tuple `(sender, receiver)`. These halves can now be passed to the corresponding threads: the `sender` goes to the thread that wants to send the messages, and the `receiver` goes to the receiving thread, respectively. There can be multiple threads that send the messages to the same channel: the `Sender` structs implements `Clone` trait, so it can be cloned and passed to as many threads as you want. The receiver, however, cannot be shared: only one thread can read the messages from the channel. 

# Event driven application 

To tie all pieces together, let's explore the high-level picture of the application: 

[Picture]

We have a main application thread that handles UI rendering. It hosts the main render loop, similar to what we saw in the [previous post][prev-post]. But now it doesn't listen for the terminal events directly. Instead, it listens for events coming from the communication channel. 
The main thread also owns the application state. Updates to the state come as events via the communication channel. Whenever the main thread receives an event, it updates the application state and re-renders the UI in the terminal. 

The download happens in the background worker thread that has access to the sender part of the channel. It informs the main thread about the download progress sending events of different kinds.

But that's only a part of the picture. Along with data update events, we also need to process the user events from the terminal. At the very least, we want to redraw the UI when the terminal window changes size. We can't do that in the main thread, because it would freeze the application. 

Instead, we create a separate thread dedicated to solely to listening for the terminal events. It shares the same channel, and transforms user input into application events that are pushed to the channel, and then received and handled by the main application loop. 

# Trying out the concept

Before diving into changing the core code, I wanted to try out the entire concept and create a simple UI application that would implement the concepts I described above. Let's suppose that at the beginning, we would like to have a simple UI that would visualize important events happening during the download process: 

1. At the beginning, I would like to show to the user that the client is probing the peers one by one; 
2. When the download process starts, I want to display the download progress: the total file size and the number of bytes downloaded. 
3. When the download process finishes, it would signal to the main loop that the work is done and the application can exit. 

#### Application events

We start carving out the data structure for application events as an enum [`AppEvent`][link]. As far as the download events are concerned, we have 3 variants: 

* `Probing(String)` with the IP address of the peer; 
* `Downloading(usize, usize)` with a pair of values `(bytes_downloaded, total_bytes)`; 
* `Completed` to signal when the download is over. 

On the other hand, the user should be able to interrupt the entire process and quite the app by pressing the Escape key on the keyboard. Also, the application should respond to the resizing of the terminal window - that comes without a question. So from the terminal input, we will expect two more kinds of events: 

* `Exit` when the user presses the Escape key; 
* `Resize` when the terminal window changes its size. 

Right now, all these events are encoded as separate variants of the `AppEvent` enum. In the future, it may make sense to make them more structured and separate _data events_ that are related to the change of the application state, from _terminal events_ that are concerned with the user interaction. For now, however, I'm happy not to complicate things too much. 

#### Application state

The application state is represented by the enum [`DownloadState`], which encodes three possible states of the application: 

* `Idle` is the default starting state; 
* `Probing(String)` as we are trying to connect to peers, with the current peer IP address; 
* `Downloading(usize, usize)` once we've started downloading: bytes downloaded and total bytes, respectively.    

It's notable that `AppEvent` and `DownloadState` look very similar. It's the case for very simple applications, but as the application grows and the UI becomes more complex, I'd expect that events and state would start to diverge considerably. 

#### Application 

The implementation resides in the [`App`][link] struct. I designated this struct to have the following responsibilities: 

1. It manages the event channel; 
2. It contains the application state; 
3. It is responsible for updating the application state in response of received events; 
4. It is responsible for processing terminal events and rendering the application state to the UI. 

There's quite a few things this struct is responsible for. In general, it's a bad smell to make a single entity responsible for so many things. I think in the future the single `App` struct will be split into several more focused entities. For example, managing the application state and applying updates to it look like a good candidate for splitting out. As a starting point, however, it works for now. 

There's two main public methods in this struct. [`start_background_task()`][link] takes a closure as a parameter and runs it in the background thread. Under the hood, it clones the event sender and passes the new sender to the provided closure, so that the background routine can send events to the main thread. This method is a point where we plug in our business logic. 

The second method [`run_ui_loop`][] is a main driver. It starts the main application loop that orchestrates the whole thing: listening for events on the communication channel, updating the application state, and rendering the UI. 

Finally, I want to see how the whole concept plays out before making changes to the core logic of the application. For starters, I'm using a fake `downoad_file` function that just simulates the real work by sending a sequence of application events with some delay.

![Screen recording]({{ site.baseurl }}/assets/images/background-tasks-ratatui/main-tui.gif)