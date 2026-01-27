---
layout: post
title:  "Ratatui: error handling in background tasks"
date: 2026-01-24
---

I was just in the middle of connecting the code of the application to the UI, when suddenly I realized that I missed a very important topic: how are we supposed to handle errors that may occur in the background task? In particular, if the download fails, how should we react to it? The UI implementation I started in the [previous post][prev-post] simply ignored the fact that a background task may fail. That realization made me backtrack a bit and reason about error handling more thoroughly. 

# Download fails, what to do? 

Now, let's think about this problem. Our main download logic is implemented in `[Torrent::download_file()]`[link], and my intention is to call this function as the background task. But, the download may fail for some reason, which is hinted by the fact that this function returns a `Result<(), Error>`. Ideally, I need some graceful solution for this situation, but our current implementation doesn't allow for one: I just didn't think about error scenarios before. Very ignorant from my part, I know. 

Considering download failures, let's explore some "easy" reactions from the application part: 

* We could simply ignore the error. That's obviously a wrong approach: if we ignore the fact that the download has failed, the background task will finish silently, and it will look like the application simply froze.

* We can panic by calling `unwrap()` or `expect()`. That's better, but not very graceful. You see, if we panic in the middle of the process, the application will quit, but the terminal will be absolutely messed up. Since Ratatui switches the terminal to the raw mode, a well-behaved application must restore the "normal" terminal mode before quitting, otherwise it will stay in the raw mode forever. If the application panics, it quits abruptly and Ratatui doesn't get a chance to do the cleanup. 

The most graceful approach would be to signal to the main application loop that there was an error in the background task. In that case, the application has a chance to do the right thing: stop the render loop, restore the terminal, and quit gracefully. Ideally, we would also like to print the error to the user and quit with an error status code, as a well-behaved application should do. 

# Background tasks that may fail

Let's tackle the background task definition first. To keep things closer to the ground, let's _expect_ that a background task can fail and change the type definitions to reflect that fact. 

#### Sending errors to the main loop

Our previous implementation attempted to make a background task [as generic as possible][link-to-prev-impl]: 

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

I want to make things more specific here. First, I noticed that we don't ever use the result of `task` function (at least for now), so let's simplify it a little bit and assume that it returns an empty value `()`. Second, let's wrap it into `Result`, to emphasize the fact that we are aware that `task` can fail for some reason. The new type definition would look like that: 

```rust
 pub fn start_background_task<F>(&self, task: F) -> thread::JoinHandle<()>
    where
        F: FnOnce(Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>>,
        F: Send + 'static,
    {
        // Implementation skipped 
    }
```

So far, so good. Now that we are aware that the `task` function returns a `Result`, we can do something meaningful if there is an error. As I said above, we would like to signal to the main loop that the background task failed, so that it can shut down gracefully. We'll do that by sending a new kind of `AppEvent`, passing the error along: 

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

To make it work, I need to introduce a new variant to the `AppEvent` enum, with the signature `Error(Box<dyn std::error::Error>)`. Arguably, I could extend the existing `AppEvent::Exit` variant to optionally pass the error with it, but I think that a separate variant would be a bit cleaner. After all, it's not obvious at all that we necessarily want to quit the application in response of an error. We could, for example, show the error in the UI and keep the app running until the user quits it explicitly. 

A few remarks on the implementation here: 

* We'll panic if we fail to send the `Error` event to the event channel. As I said before, panicking in Ratatui applications is not the best approach, but I don't see any meaningful way to handle that situation. After all, `send()` can fail only if the receiver has shut down, which means we've already quit the main application loop.

* I had to slightly change the signature of `task` function to accept a reference to `Sender`, instead of passing the ownership. That's because we still need the access to the `event_sender` after we call the `task`, so that we can send an error. 

#### Handling errors in the main loop

We need to make the changes on the receiving end in the main application loop, too. What I want to do is, when receiving `AppEvent::Error` event, the main loop should shut down gracefully and return the `Result` with the error object that caused the shutdown. Luckily, it's pretty straightforward: 

```rust
fn process_app_event(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
    match self.event_receiver.recv()? {
        // ... skipped all other event handling
        AppEvent::Error(err) => Err(err),
    }
}
```

That simple change makes us return an error `Result` from `process_app_event()`, which in turn is propagated through `run_ui_loop()` function and eventually ends up in the `main` function, where we can print the error to the user, finishing the journey of the background task error.  

So let's recap the way the error travels through the application: 

1. The background task fails and returns an error in the `Result`; 
2. The error is packed into the `AppEvent::Error` object and is sent via the event channel to the main loop; 
3. Main loop receives the error event and shut downs its operation, making `App::run_ui_loop()` return the `Result` with that same error; 
4. The error ends up in the `main` function that prints the error to the terminal and exits with a failure status code. 

Is that all we need to do? Well, not so fast: even though conceptually everything is correct, the program doesn't compile! It turns out, in a  multi-threaded application, we need to take care of a few more details. 

# Respecting the threads: _Send_ and _Sync_ traits 

The code I just wrote would work just fine if we didn't have to send `AppState` instances via the channel between threads. You see, in order for a variable to be allowed to be moved between threads, it needs to implement the [`Send`] trait. If the type is not `Send`, you can't pass it between threads, i.e. you'll get an error if your `thread::spawn()` code tries to move that variable to another thread.

`Send` is a special kind of trait, called a _marker trait_. Marker traits don't contain any methods. Instead, their purpose is to convey some specific information to the compiler. In case of `Send`, the compiler knows that the values of a type that implements `Send` are safe to pass between threads. 

Usually, you don't need to implement `Send` on your types yourself. `Send` is also an _auto-trait_: the compiler automatically implements an auto-trait for types, unless the type contains something that doesn't implement the marker trait. So in case of `Send`, the type will automatically be `Send` if all its fields implement `Send`. 

Which brings us to the `AppState` type and the latest addition of the variant: 

```rust 
pub enum AppEvent {
    Error(Box<dyn std::error::Error>),
    // ...other variants skipped
}
```

Type-wise, this addition makes our whole `AppEvent` not `Send`. Let's explore why. `Box` will be `Send` only if its inner value is `Send`. However, in our type definition we don't restrict this fact. We only declare that the value `Box` holds must implement `std::error::Error` trait. Potentially, that means that we can create the `Error` variant that would hold a value to some error that implements `std::error::Error` trait, but is not `Send`-compatible: for example, the value can hold a shared reference `Rc<T>` as one of it fields, which makes that value not safe to pass between threads. 

The solution is to restrict the types that `AppEvent::Error` can wrap around. We must ensure that the inner value is also `Send`, by providing a more specific type boundary: 

```rust 
pub enum AppEvent {
    Error(Box<dyn std::error::Error + Send>),
    // ...other variants skipped
}
```

Technically, that should be enough to satisfy the compiler. However, after we go through the source code and change all relevant `Result`s to `Result<T, Box<dyn std::error::Error + Send>>` to 











