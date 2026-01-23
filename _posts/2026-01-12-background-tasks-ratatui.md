---
layout: post
title:  "Ratatui: working with background tasks"
date: 2026-01-12
---

In the [previous post][prev-post] I explored a structure of a simple interactive Ratatui application. However, the BitTorrent client presents additional challenges: the driver of the application is the download process that works outside the UI-driven render loop. In this section, I'm laying the groundwork for a terminal UI application that does most of its work in the background. 

# The problem 

A typical interactive application performs most of it's tasks as a response to the user's actions, so it's primarily driven by the application render loop that only listens and reacts to the terminal events, as we saw in the [previous post][prev-post]. 

A BitTorrent client is different, though. Essentially, the application runs its own course without any input from the user: instead, the most interesting stuff in that application is driven by the inner logic of communication with the remote host. 

Along with that, the application needs to remain responsive to the user input while the download process is ongoing. We should respond to the user input, for example if the user decides to quit the application by pressing some key combination, or resizes the terminal window, etc.

Generally speaking, there are two _logical execution threads_ that run in parallel in the application: one is performing the download task, and another is reacting to the user's events. Both of these threads should have the ability to trigger the updates to the UI. 

There are different ways to implement an application with multiple logical execution threads. The most obvious one is of course using the physical threads provided by the operating system. Another approach, which is quite natural for IO-heavy applications is _asynchronous programming_, which allows us to separate the logical execution threads from the physical threads. 

Both approaches can be implemented in Rust, but I don't feel comfortable yet diving into the specifics of Rust's asynchronous programming. So for the time being, I'm going to proceed with a more straightforward multi-threaded solution. 

For now, I'm going to proceed with a simple multi-threaded solution. 

# Event-driven multi-threaded application 

The solution I'd like to implement looks as follows: 

[Picture]

When application starts, we spawn two execution threads. One thread is dedicated to handling the entire file download process. Another thread is simply listening to the terminal events. Both threads have the access to the shared _communication channel_, to which they send _application events_ when something interesting has happened on their side. 

#### Download thread

From the download thread, we'd like to notify the user about what's currently going on: 

* When probing peers one by one, we'd like to notify the user about which peer we're currently connecting to; 
* When the actual downloading starts, we'd like to update the user each time we receive a portion of the file. 
* When the download has finished, we'd like to quit the application.

#### User interaction thread

While the download thread is busy downloading the file, another thread is listening to the terminal events to react to user actions: 

* When the user presses `Escape` key, we'd like to interrupt the process and quit the application; 
* If the user resizes the terminal window, we'd like to redraw the UI to fit the new window size. 

Essentially, the task of that thread is to transform interesting terminal events into corresponding application events. Neither thread is interacting with the terminal directly: instead, they just push events of different kinds into the communication channel. 

On the receiving end of the communication channel is our main application render loop that runs in the main application thread. It's task is similar to what we [saw before][prev-post-app-loop], except that now it listens to the events from the communication channel: 

* It blocks until a new event appears in the channel;
* It processes the event by updating the application state, if needed; 
* It re-renders the UI with the updated application state. 

The application loop terminates when it receives a certain event: either the download has finished, or the user wants to quit the application by pressing `Escape` button. 

#### Inter-thread communication channel

Now, it's obvious that we need an implementation for the communication channel, via which the background threads will send the events to the loop. Luckily, message-passing inter-thread communication is a very common task, and Rust's standard library provides provides an implementation for such communication in [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html) module. By the way, "mpcs" is an abbreviation for "**M**ultiple **P**roducers, **S**ingle **S**ubscriber": multiple threads can send messages to the channel, but only one thread is allowed to receive them.

The function [`channel()`](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html) creates an asynchronous communication channel, returning a tuple `(sender, receiver)`. These halves can now be passed to the corresponding threads: the `sender` goes to the thread that wants to send the messages, and the `receiver` goes to the receiving thread, respectively. 

There can be multiple threads that send the messages to the same channel: the `Sender` structs implements `Clone` trait, so it can be cloned and passed to as many threads as you want. The receiver, however, cannot be shared: only one thread can read the messages from the channel. 
 
# Trying out the concept

Before diving into changing the core code, I wanted to try out the entire concept and create a simple UI application that would implement the concepts I described above. Let's suppose that at the beginning we would like to have a simple UI that visualizes important events happening during the download process: 

1. At start, I would like to show to the user that the client is probing the peers one by one; 
2. When the download process starts, I want to display the download progress: the total file size and the number of bytes downloaded. 
3. When the download process finishes, it would signal to the main loop that the work is done and the application can exit. 

#### Application events

We start carving out the data structure for application events as an enum [`AppEvent`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L16). As far as the download events are concerned, we have 3 variants: 

* `Probing(String)` with the IP address of the peer; 
* `Downloading(usize, usize)` with a pair of values `(bytes_downloaded, total_bytes)`; 
* `Completed` to signal when the download is over. 

On the other hand, the user should be able to interrupt the entire process and quite the app by pressing the Escape key on the keyboard. Also, the application should respond to the resizing of the terminal window - that comes without a question. So from the terminal input, we will expect two more kinds of events: 

* `Exit` when the user presses the Escape key; 
* `Resize` when the terminal window changes its size. 

Right now, all these events are encoded as separate variants of the `AppEvent` enum. In the future, it may make sense to make them more structured and separate _data events_ that are related to the change of the application state, from _terminal events_ that are concerned with the user interaction. So far, however, I'm happy not to complicate things too much. 

#### Application state

The application state is represented by the enum [`DownloadState`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L31), which encodes three possible states of the application: 

* `Idle` is the default starting state; 
* `Probing(String)` as we are trying to connect to peers, with the current peer IP address; 
* `Downloading(usize, usize)` once we've started downloading: bytes downloaded and total bytes, respectively.    

It's notable that `AppEvent` and `DownloadState` look very similar. It's the case for very simple applications, but as the application grows and the UI becomes more complex, I'd expect that events and state would start to diverge considerably. 

#### Application logic

The implementation resides in the [`App`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L24) struct. I designated this struct to have the following responsibilities: 

1. It manages the event channel; 
2. It contains the application state; 
3. It is responsible for updating the application state in response of received events; 
4. It is responsible for processing terminal events and rendering the application state to the UI. 

There's quite a few things this struct is responsible for. In general, it's a bad smell to make a single entity responsible for so many things. I think in the future the single `App` struct will be split into several more focused entities. For example, managing the application state and applying updates to it look like a good candidate for splitting out. As a starting point, however, it works for now. 

There are two main public methods in this struct. [`start_background_task()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L49) takes a closure as a parameter and runs it in the background thread. Under the hood, it clones the event sender and passes the new sender to the provided closure, so that the background routine can send events to the main thread. This method is a point where we plug in our business logic. 

The second method [`run_ui_loop`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L59) is a main driver. It starts the main application loop that orchestrates the whole thing: listening for events on the communication channel, updating the application state, and rendering the UI. 

# Fake it till you make it

I want to see how the whole concept plays out. It would be nice to see that UI parts play well together before I dive into refactoring  the core logic of the application. 

To give me a nice playground, I've created a separate binary target [`main-tui.rs`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/bin/main-tui.rs) that I'll be using to try out the UI part of the program. 

To simulate the real work, I'm using a fake [`downoad_file()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/bin/main-tui.rs#L13): 

```rust
fn download_file(tx: Sender<AppEvent>) {
    let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"];
    for ip_address in ip_addresses {
        tx.send(AppEvent::Probing(ip_address.to_string())).unwrap();
        thread::sleep(Duration::from_secs(2));
    }

    for i in 0..100 {
        tx.send(AppEvent::Downloading(i, 100)).unwrap();
        thread::sleep(Duration::from_millis(100));
    }

    tx.send(AppEvent::Completed).unwrap();
}
``` 

This dummy function imitates the real download process by sending application events with some delay. 

Let's see what it looks like in the terminal! When I compile and run `main-tui.rs`, I can see the following output: 

![Screen recording]({{ site.baseurl }}/assets/images/background-tasks-ratatui/main-tui.gif)

Excellent! The UI looks clumsy, for sure, but the core behaviour is there: we can see what the downloader is doing behind the curtains. The application is also responsive to the user input: though not shown in this animation, it handles it nicely when I resize the terminal window. I can also interrupt the entire process by pressing the Escape button, as nicely hinted in the user interface. 

# Next steps 

By now, I've created and tried out the base framework to introduce a nice Ratatui user interface to my BitTorrent client: the solution with several background threads works. Next, I'm going to connect the UI part to the core logic and add some polish to the user interface.  

[prev-post]: {{site.baseurl}}/{% post_url 2025-12-27-starting-with-ratatui %}