---
layout: post
title:  "Non-blocking I/O: request the file from a peer"
date: 2026-04-10 
---

Working with non-blocking I/O is in fact a challenge for a developer. In fact, we need to move away from a straightforward sequential coding and start treating every I/O exchange as a _state machine_, driven by the events that come from I/O event queue. 

Let's have a look at the message exchange we need to do with the peer before we start downloading: 

![Request file message exchange]({{ site.baseurl }}/assets/images/non-blocking-request-file/request-file-sequence.svg)

This is how that message exchange looks when modeled as a state machine: 

![Probe state diagram]({{ site.baseurl }}/assets/images/non-blocking-request-file/probe-state-diagram.svg)



