---
layout: post
title:  "Ratatui: error handling in background tasks"
date: 2026-01-29
---

I was just in the middle of connecting the code of the application to the UI, when suddenly I realized that I skipped a very important topic: **how are we supposed to handle errors that may occur in the background task?** In particular, if the download fails, how should we react to it? The UI implementation I started in the [previous post][prev-post] simply ignored the fact that a background task can fail. That realization made me backtrack a bit and reason about error handling more thoroughly. 

[*Version 0.0.14 on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.14){: .no-github-icon}

# Download fails, what to do? 

Now, let's think about this problem. Our main download logic is implemented in [`Torrent::download()`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.14/src/lib.rs#L21), and my intention is to call this function as the background task. But the download may fail for some reason, which is hinted by the fact that this function returns a `Result<(), Error>`. Ideally, I need some graceful solution for this situation, but our current implementation doesn't allow for one: I just didn't think about error scenarios before. Very ignorant on my part, I know. 

Considering download failures, let's explore some "easy" reactions from the application's perspective: 

* We could simply ignore the error. That's obviously a wrong approach: if we ignore the fact that the download has failed, the background task will finish silently, and stop updating the UI. It will look like the application simply froze.

* We can panic by calling `unwrap()` or `expect()`. That's better, but not very graceful. If we panic mid-way, the application will quit, but the terminal will be absolutely messed up. Since Ratatui switches the terminal to the raw mode, a well-behaved application must restore the "normal" terminal mode before quitting, otherwise it will stay in the raw mode forever. When panicked, the application quits abruptly and Ratatui doesn't get a chance to do the cleanup. 

The most graceful approach would be to signal to the main application loop that there was an error in the background task. In that case, it has a chance to do the right thing: stop the render loop, restore the terminal, and quit cleanly. Ideally, we would also like to print the error to the user and quit with an error status code, as a well-behaved application should do. 

# Background tasks that may fail

Let's tackle the background task definition first. To keep things closer to the ground, let's _expect_ that a background task can fail and change the type definitions to reflect that fact. 

#### Sending errors to the main loop

Our previous implementation attempted to make a background task [as generic as possible](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.13/src/ratatui_ui.rs#L49): 

```rust 
pub fn start_background_task<F, T>(&self, task: F) -> thread::JoinHandle<T>
    where
        F: FnOnce(Sender<AppEvent>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // Implementation skipped for clarity
    }
```

I want to make things more specific here. First, I noticed that we never use the result of `task` function (at least for now), so let's simplify it a little bit and assume that it returns an empty value `()`. Second, let's wrap it into `Result`, to emphasize the fact that we are aware that `task` can fail for some reason. The new type definition would look like this: 

```rust
 pub fn start_background_task<F>(&self, task: F) -> thread::JoinHandle<()>
    where
        F: FnOnce(Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>>,
        F: Send + 'static,
    {
        // Implementation skipped 
    }
```

So far, so good. Now that we are aware that the `task` function returns a `Result`, we can do something meaningful in case of an error. As I said above, we would like to signal to the main loop that the background task failed, so that it can shut down gracefully. We'll do that by sending a new kind of `AppEvent`, passing the error along: 

```rust
pub fn start_background_task<F>(&self, task: F) -> thread::JoinHandle<()>
    where
        F: FnOnce(&Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>>,
        F: Send + 'static,
    {
        let event_sender = self.event_sender.clone();
        thread::spawn(move || {
            if let Err(err) = task(&event_sender) {
                let error_msg = format!("failed to send error event: {:?}", err);
                event_sender.send(AppEvent::Error(err)).expect(&error_msg);
            }
        })        
    }
```

To make it work, I need to introduce a new variant `AppEvent::Error(Box<dyn std::error::Error>)`. Arguably, I could extend the existing `AppEvent::Exit` variant to optionally pass the error with it, but I think that a separate variant would be a bit cleaner. After all, it's not universal that we want to quit the application in response to an error. We could, for example, show the error in the UI and keep the app running until the user quits it explicitly. 

A few remarks on the implementation here: 

* We'll panic if we fail to send the `Error` event to the event channel. As I said before, panicking in Ratatui applications is not the best approach, but I don't see any meaningful way to handle that situation. After all, `send()` can fail only if the receiver has shut down, which means we've already quit the main application loop.

* I had to slightly change the signature of `task` function to accept a reference to `Sender`, instead of passing the ownership. That's because we still need the access to the `event_sender` after we call the `task`, so that we can send an error. 

#### Handling errors in the main loop

We need to make the changes on the receiving end in the main application loop, too. 

What I want to do is, when receiving `AppEvent::Error` event, the main loop should shut down gracefully and return the `Result` with the error object that caused the shutdown. Luckily, it's pretty straightforward: 

```rust
fn process_app_event(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
    match self.event_receiver.recv()? {
        // ... skipped all other event handling
        AppEvent::Error(err) => Err(err),
    }
}
```

That simple change makes us return an error `Result` from `process_app_event()`, which in turn is propagated through `run_ui_loop()` function and eventually ends up in the `main` function, where we can print the error to the user, finishing the journey of the background task error.  

#### Summary: the error path

So let's recap the way the error travels through the application: 

1. The background task function fails and returns an error in the `Result`; 
2. The error is packed into the `AppEvent::Error` object and is sent via the event channel to the main loop; 
3. Main loop receives the error event and shuts down its operation, making `App::run_ui_loop()` return the `Result` with that same error; 
4. The error ends up in the `main` function that prints the error to the terminal and exits with a failure status code. 

Is that all we need to do? Well, not so fast: even though conceptually everything is correct, the program doesn't compile! It turns out, in a multi-threaded application, we need to take care of a few more details. 

# Respecting the threads: _Send_ and _Sync_ traits 

The code I just wrote would work just fine if we didn't have to send `AppEvent` instances via the channel between threads. In order for a variable to be "movable" between threads, it needs to implement the [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) trait. If the type is not `Send`, you can't pass it between threads, i.e. the compiler will complain if your `thread::spawn()` code tries to move that variable to another thread.

`Send` is a special kind of trait, called a _marker trait_. Marker traits don't contain any methods. Instead, their purpose is to convey some specific information to the compiler. In case of `Send`, the compiler knows that the values of a type that implements `Send` are safe to pass between threads. 

Usually, you don't need to implement `Send` on your types yourself. `Send` is also an _auto-trait_: the compiler automatically implements an auto-trait for custom types, unless the type contains something that doesn't implement that same auto-trait. So in case of `Send`, the type will automatically be `Send` if all its fields implement `Send`. 

Which brings us to the `AppEvent` type and the latest addition of the variant: 

```rust 
pub enum AppEvent {
    Error(Box<dyn std::error::Error>),
    // ...other variants skipped
}
```

Type-wise, this addition makes our whole `AppEvent` not `Send`. Let's explore why. `Box` will be `Send` only if its inner value is `Send`. However, in our type definition we don't restrict this fact. We only declare that the inner value of `Box` must implement `std::error::Error` trait. Potentially, that means we can create an `Error` variant holding an error that implements `std::error::Error` but is not `Send`-compatible. For example, the value could hold a shared reference `Rc<T>` as one of its fields (which is not `Send`), making it unsafe to pass between threads. 

The solution is to restrict the types that `AppEvent::Error` can wrap around. We must ensure that the inner value is also `Send`, by providing a more specific trait bound: 

```rust 
pub enum AppEvent {
    Error(Box<dyn std::error::Error + Send>),
    // ...other variants skipped
}
```

#### More trait bounds

Technically, what we just did should be enough to satisfy the compiler. However, after we go through the source code and change all relevant return types to `Result<T, Box<dyn std::error::Error + Send>>`, we end up with multiple compiler errors throughout the code where we used `?` error propagation operator: 

```
`?` couldn't convert the error to `Box<dyn std::error::Error + Send>`
```

To answer what's breaking here we need to look a bit under the hood of the `?` operator. In my previous project [I described](https://www.tindandelion.com/rust-text-compression/2025/05/01/tidbits-of-error-handling.html) that `?` operator implicitly converts between error types using `From` trait. That makes error propagation work seamlessly, as long as Rust knows how to convert one error type into the other. So why does it work with `Result<T, Box<dyn Error>>` but breaks for `Result<T, Box<dyn Error + Send>>`? 

Rust's standard library contains [a blanket implementation](https://doc.rust-lang.org/std/error/trait.Error.html#impl-From%3CE%3E-for-Box%3Cdyn+Error%3E) to convert from `Error` to `Box<dyn Error>`: 

```rust 
impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a>
```

However, as the compiler error tells us, it knows nothing about converting to `Box<Error + Send>`. My first impulse was to provide such an implementation, but unfortunately that doesn't work either: `From`, `Error` and `Box` are _foreign types_, that is, they are both defined outside my local crate. When it comes to trait implementation, Rust has a so-called _orphan rule:_ you can implement a trait for a type only if the trait or the type is local to your crate. 

So we can't provide a blanket implementation to satisfy the compiler. Are we stuck? 

Well, it turns out that Rust's standard library gives us a way out. There's [another blanket implementation](https://doc.rust-lang.org/std/error/trait.Error.html#impl-From%3CE%3E-for-Box%3Cdyn+Error+%2B+Send+%2B+Sync%3E) of `From` that can help us, with a stricter bound: 

```rust 
impl<'a, E: Error + Send + Sync + 'a> From<E> for Box<dyn Error + Send + Sync + 'a>
```

Notice that there's a trait [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html) mentioned in the type bound. This is another thread-related marker trait we should be aware about. 

`Sync` marks types whose references `&T` can be safely accessed from multiple threads without a threat of data races. Like with `Send`, the compiler automatically implements this trait for custom types whose fields are all `Sync`, which includes primitive types (and some other thread-related types, like `Mutex<T>`). The types that are **not** `Sync` include `Cell<T>`, `RefCell<T>` and `Rc<T>`.

To summarize both `Send` and `Sync`: 

* `Send` means that the ownership can be transferred to another thread; 
* `Sync` means that references can be shared across threads. 

Long story short, it seems that to make the code work without compiler errors, we need to adjust our definition of `AppEvent`: 

```rust 
pub enum AppEvent {
    Error(Box<dyn std::error::Error + Send + Sync>),
    // ...other variants skipped
}
```

And, when defining opaque errors, we need to use `Result<T, Box<dyn std::error::Error + Send + Sync>>` to ensure thread safety for error types. Since it's quite a long definition that's going to appear quite frequently in the code, I've extracted helpful type aliases into [`result.rs`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.14/src/result.rs) local module, for easier use. Luckily, besides changing the type definitions, no other changes are required: all error types we've used so far comply with `Send` and `Sync` restrictions. 

That's essentially a bottom line when it comes to passing errors across threads. 

# Putting it all together

Let's now try out how our application reacts to errors in reality. To test the approach, I've created a new example [`examples/tui-app-error.rs`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.14/examples/tui-app-error.rs) to simulate an IO error that happens during the download. 

Running this example, we get the following output: 

![Error handling output]({{ site.baseurl }}/assets/images/error-handling-background-tasks/tui-app-error.gif)

Very well! Now the application terminates gracefully without breaking the terminal, and we see the error description in the console. 

Granted, the error description looks a bit short and doesn't provide a lot of context to help troubleshooting. I'm actually planning to dive deeper into how to make error messages more helpful later in the project. For now, it works for me. 

# Moving on

This post was a bit of a digression from what I [planned to do][prev-post-plan] initially, but it was very important to address the subject of graceful error handling to move forward. 

Now, I'm back to my plan and I feel ready to finally connect the core logic of the BitTorrent client to the UI, and make the interface a bit fancier, utilizing some nice widgets that Ratatui provides. Stick around! 

[*Current version (0.0.14) on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.0.14){: .no-github-icon}

[prev-post]: {{site.baseurl}}/{% post_url 2026-01-12-background-tasks-ratatui %}
[prev-post-plan]: {{site.baseurl}}/{% post_url 2026-01-12-background-tasks-ratatui %}#next-steps












