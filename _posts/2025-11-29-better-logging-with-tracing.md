---
layout: post
title:  "Better logging with Tracing"
date: 2025-11-29
---

Another thing that has interested me for a while was how to approach logging in a Rust application. Until now, I was just relying on `println()` macro to display significant events in my torrent client. However, this is not a sustainable approach: in real-world applications, you don't want `println()` statements here and there: your approach to logging should be more systematic. 

Usually in applications, developers rely on some sort of a _logging library_ that offers developers a flexible and configurable approach to manage logs. I was unaware of such libraries in Rust, until I came across an excellent video by Jon Gjengset, [Decrusting the tracing crate](https://youtu.be/21rtHinFA40?si=yHWqdFj0j08thUo1), that introduced me to the [Tracing](https://docs.rs/tracing/latest/tracing/) framework and gave me the answer I needed. 

# Using tracing in my project 

As described [in the documentation](link), Tracing provides a developer with powerful capabilities to keep track of what's going on in the application using spans and events. I can see how it can be useful to record useful information on many levels in my application: 

* On the low level, we can keep track of the message exchange with the remote peer: what messages we send to the peer and what we receive from it; 
* On a higher level, we can record the process of downloading the file piece by piece. Since downloading a single piece is also done in separate blocks, it makes sense to keep track of requesting and receiving individual blocks as well. 

At this time, however, I want to keep things simple. My goal is to get rid of `println()` statements in the code, and replace them with appropriate `info()` and `debug()` macros from Tracing. 

```ansi-output 
[2m2025-12-01T08:07:37.251354Z[0m [34mDEBUG[0m [2mreqwest::connect[0m[2m:[0m starting new connection: http://bttracker.debian.org:6969/
[2m2025-12-01T08:07:37.254314Z[0m [34mDEBUG[0m [2mhyper_util::client::legacy::connect::http[0m[2m:[0m connecting to 130.239.18.158:6969
[2m2025-12-01T08:07:37.326947Z[0m [34mDEBUG[0m [2mhyper_util::client::legacy::connect::http[0m[2m:[0m connected to 130.239.18.158:6969
[2m2025-12-01T08:07:37.431714Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Received peer addresses [3mpeer_count[0m[2m=[0m50
[2m2025-12-01T08:07:37.431791Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Probing peers
[2m2025-12-01T08:07:37.431900Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m77.33.175.70:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-01T08:07:42.432966Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m77.33.175.70:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Failed to connect [3merror[0m[2m=[0mconnection timed out
[2m2025-12-01T08:07:42.433680Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m103.69.224.26:56400[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-01T08:07:42.826996Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m103.69.224.26:56400[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Failed to connect [3merror[0m[2m=[0mfailed to fill whole buffer
[2m2025-12-01T08:07:42.827198Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m176.10.224.10:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-01T08:07:43.483832Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m176.10.224.10:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connected [3mremote_id[0m[2m=[0m-TR4060-o1tenw8zlqbs
[2m2025-12-01T08:07:43.483950Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m176.10.224.10:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connected, requesting file
[2m2025-12-01T08:07:52.388603Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m176.10.224.10:51413[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Ready to download
[2m2025-12-01T08:07:52.388809Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Downloading file [3mfile_size[0m[2m=[0m702545920 [3mpiece_count[0m[2m=[0m2680 [3mpeer_address[0m[2m=[0m176.10.224.10:51413 [3mremote_id[0m[2m=[0m-TR4060-o1tenw8zlqbs
[2m2025-12-01T08:07:53.269920Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m0 [3mduration_ms[0m[2m=[0m879
[2m2025-12-01T08:07:53.516152Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m1 [3mduration_ms[0m[2m=[0m246
[2m2025-12-01T08:07:53.727754Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2 [3mduration_ms[0m[2m=[0m211
[2m2025-12-01T08:07:53.937935Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m3 [3mduration_ms[0m[2m=[0m210
[2m2025-12-01T08:07:54.208434Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m4 [3mduration_ms[0m[2m=[0m270
[2m2025-12-01T08:07:54.388954Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m5 [3mduration_ms[0m[2m=[0m180
[2m2025-12-01T08:07:54.633477Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m6 [3mduration_ms[0m[2m=[0m244
[2m2025-12-01T08:07:54.911209Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m7 [3mduration_ms[0m[2m=[0m277

```

<script type="module">
import { AnsiUp } from 'https://cdn.jsdelivr.net/npm/ansi_up@6.0.2/ansi_up.min.js';

document.addEventListener('DOMContentLoaded', function() {
    const ansi_up = new AnsiUp();
    document.querySelectorAll('code.language-ansi-output').forEach(function(block) {
        const text = block.textContent;
        block.innerHTML = ansi_up.ansi_to_html(text);
    });
});
</script>
