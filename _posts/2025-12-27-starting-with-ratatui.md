---
layout: post
title:  "Starting with Ratatui: the UI for BitTorrent client"
date: 2025-12-27
---

TODO: Small summary 

# Approaching the application user interface 

Let's consider a few possible approaches to building the application's user interface: 

* The first option that comes to mind is to outsource the UI work completely into a separate project. Our client could provide a Web API to query the download progress. There is a bunch of Web frameworks to build Web servers in Rust ([Axum](https://docs.rs/axum/latest/axum/), [Actix Web](https://actix.rs/)).

* The second approach would be to build a fully-fledged desktop application using [Tauri](https://v2.tauri.app/). Tauri is a framework for building cross-platform applications for desktop and mobile platforms. With Tauri, you build your application logic in Rust, and the UI layer using any modern Web front-end framework, such as React or Vue.js. As far as I understand the technology underneath, it is similar to [Electron](https://www.electronjs.org/) for JavaScript/TypeScript desktop applications. 

* Finally, we can stay completely in the terminal world and still build a rich UI application: so-called _Terminal UI application_, or _TUI_ for short. There are a few frameworks and libraries for Rust that help you build rich TUI applications, most popular currently being [Ratatui](https://ratatui.rs/). 

All of these options look very compelling to me, each one providing a lot of opportunities to learn. The first two, however, would require me to leave Rust land for the front-end work. I'd like to stay in the Rust ecosystem for now, so I decided to move forward with the third option: build a UI layer for my BitTorrent client fully in the terminal, with the help of Ratatui library. 

# Hello World in Ratatui 

The first thing to notice about Ratatui is that it's a _library_ of useful tools, not a _UI framework_. What's the difference? Well, a framework would usually handle the entire application lifecycle, providing the developer with the extension points to plug in your specific business logic. The framework usually hides some pesky low-level details from the developer and minimizes the amount of boilerplate code irrelevant to the application's business logic. 

You won't see that with Ratatui. Rather, Ratatui is a toolkit of useful UI abstractions that work on top of the [_terminal backend_ library](https://ratatui.rs/concepts/backends/). The terminal backend library provides an API to manipulate the terminal on a very low level: display text in different colors and styles, read events like keystrokes or mouse clicks, etc. On top of that, Ratatui adds a higher level of abstractions that allow you to work with the terminal in terms of _widgets_: the building blocks of your application's user interface. However, it's the developer's responsibility to write the code that handles the application lifecycle and binds different pieces together. 

The upside of such a lightweight approach is that the library doesn't confine you to a particular application apadigm or architecture: you're free to choose whatever style you like. The downside, though, is that the developer is responsible for writing a bit of a boilerplate code to manage some low-level details, that would otherwise be provided by the UI framework. 

Let's have a look at the most basic [_Hello world_ application](link to example code) in Ratatui and explore the important parts: 

```rust
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

pub fn main() {
    // 1: Initialize the terminal
    let mut terminal = ratatui::init();
    // 2: Enter the render loop
    loop {
        // 2.1: Render the UI
        terminal
            .draw(|frame| {
                let text = Line::from("Hello, world!").bold().italic();
                let widget = Paragraph::new(text).centered().block(Block::bordered());
                frame.render_widget(widget, frame.area());
            })
            .expect("failed to draw frame");
        // 2.2: Wait for user input
        match event::read().expect("failed to read event") {
            Event::Key(key) => {
                if key.code == KeyCode::Esc {
                    break;
                }
            }
            _ => (),
        }
    }
    // 3: Restore the terminal
    ratatui::restore();
}
```



--- 



# Mental model of an interactive application 

Interactive applications, as the name implies, are application that are driven by the interactions with the user. When developing such an application, it's worth keeping in mind a simple mental model: 

![Interactive application loop]({{ site.baseurl }}/assets/images/starting-with-ratatui/interactive-app-loop.svg)

At the center of the application is a _render loop_ that goes through 3 main steps: 
* it renders the current application state in the terminal; 
* it waits for the next user event to occur: keyboard clicks, mouse events, etc. 
* it updates the application state based on the user input. 
* if the user wishes to quit the application, the loop is terminated. Otherwise, we go to the next iteration. 

In the most basic form, we can create an interactive application using solely standard terminal input/output. The application could print the current state in some textual representation, and expect the user input 





# The structure for Ratatui application 

- Application state
- Render loop that displays the application state to the user

Typical interactive application: 

* Render the application state; 
* Read events from the terminal (user input, mouse events, resize events, etc.);
* Update application state according to the user input.

![Interactive application loop]({{ site.baseurl }}/assets/images/starting-with-ratatui/interactive-app-loop.svg)

In our case, however, the situation is different. We don't have an interactive application per se. Rather, we have a lengthy (?)process that runs its course and at certain moments it updates the application state. The UI should re-render itself whenever the application state changes. But there's also a limited interaction with the the user through the UI. We would like the user to be able to interrupt the application by pressing a specific key (ESC). Also, we would like the application to redraw itself when the user resizes the terminal window. 

[Picture for event driven UI]


