---
layout: post
title:  "Ratatui: working with background tasks"
date: 2026-01-12
---

In the [previous post][prev-post] I explored a structure for a typical interactive Ratatui application. However, a BitTorrent client presents additional challenges: the meat of the application is the download process that happens outside a UI-driven render loop. In this section, I'm laying the groundwork for a terminal UI application that does most of its work in the background. 

![Screen recording]({{ site.baseurl }}/assets/images/background-tasks-ratatui/main-tui.gif)