---
layout: post
title:  "Starting with Ratatui: the UI for BitTorrent client"
date: 2025-12-27
---

TODO: Small summary 

# UI alternatives 

* The first option that comes to mind is to outsource the UI work completely into a separate project. Our client could provide a Web API to query the download progress. There is a bunch of Web frameworks to build Web servers in Rust ([Axum](https://docs.rs/axum/latest/axum/), [Actix Web](https://actix.rs/)).

* The second option would be to build a fully-fledged desktop application using [Tauri](https://v2.tauri.app/). Tauri is a framework for building cross-platform applications for desktop and mobile platforms. With Tauri, you build your application logic in Rust, and the UI layer using any modern Web front-end framework, such as React or Vue.js. As far as I understand the technology underneath, it is similar to [Electron](https://www.electronjs.org/) for JavaScript/TypeScript desktop applications. 

* Finally, we can stay completely in the terminal world and still build a rich UI application: so-called _Terminal UI application_, or _TUI_ for short. There are a few frameworks and libraries for Rust that help you build rich TUI applications, most popular currently being [Ratatui](https://ratatui.rs/). 

All of these options look very compelling to me, each one providing a lot of opportunities to learn. The first two, however, would require me to leave Rust land for the front-end work. I'd like to stay in the Rust ecosystem for now, so I decided to move forward with the third option: build a UI layer for my BitTorrent client fully in the terminal, with the help of Ratatui library. 

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


