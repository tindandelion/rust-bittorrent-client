---
layout: post
title:  "Better logging with Tracing"
date: 2025-11-29
---

Yet another thing that has interested me for a while was how to approach logging in a Rust application. Until now, I was just relying on `println()` macro to display significant events in my torrent client. However, this is a poor man solution: in real-world applications, you don't want `println()` statements here and there: your approach to logging should be more systematic. 

Usually in applications, developers rely on some sort of a _logging library_ that offers developers flexible and configurable means to manage logs. I was unaware of such libraries in Rust, until I came across an excellent video by Jon Gjengset, [Decrusting the tracing crate](https://youtu.be/21rtHinFA40?si=yHWqdFj0j08thUo1), that introduced me to the [Tracing](https://docs.rs/tracing/latest/tracing/) framework and gave me the answer I needed. 

# Using tracing in my project 

As described [in the documentation](https://tracing.rs/tracing/), Tracing provides a developer with powerful capabilities to keep track of what's going on in the application using spans and events. I can imagine how it can be useful to record useful information on many levels in my application: 

* On the low level, we can keep track of the message exchange with the remote peer: what messages we send to the peer and what we receive from it; 
* On a higher level, we can record the process of downloading the file piece by piece. Since downloading a single piece is also done in separate blocks, it makes sense to keep track of requesting and receiving individual blocks as well. 

Using Tracing, we can collect the diagnostic information from top to bottom, as a nice hierarchy of spans. At this time, however, I want to keep things simple. My goal is to get rid of `println()` statements in the code, and replace them with appropriate `info()` and `debug()` macros from Tracing. 

I'm also not going to work much on consuming the events yet. There's a [plethora of implementations](https://tracing.rs/tracing/#related-crates) in Tracing ecosystem that allows you to send collected traces to OpenTelemetry, Sentry, etc. For now, however, I'm quite satisfied with the ready-to-use implementation from [`tracing_subscriber`](https://docs.rs/tracing-subscriber/0.3.22/tracing_subscriber/fmt/index.html) crate, which prints events to the console.

```ansi-output 
[2m2025-12-02T06:21:03.526813Z[0m [34mDEBUG[0m [2mreqwest::connect[0m[2m:[0m starting new connection: http://bttracker.debian.org:6969/
[2m2025-12-02T06:21:03.529348Z[0m [34mDEBUG[0m [2mhyper_util::client::legacy::connect::http[0m[2m:[0m connecting to 130.239.18.158:6969
[2m2025-12-02T06:21:03.672770Z[0m [34mDEBUG[0m [2mhyper_util::client::legacy::connect::http[0m[2m:[0m connected to 130.239.18.158:6969
[2m2025-12-02T06:21:03.757824Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Received peer addresses [3mpeer_count[0m[2m=[0m50
[2m2025-12-02T06:21:03.757899Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Probing peers
[2m2025-12-02T06:21:03.757996Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m194.15.111.74:53817[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-02T06:21:08.759082Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m194.15.111.74:53817[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Failed to connect [3merror[0m[2m=[0mconnection timed out
[2m2025-12-02T06:21:08.759370Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m203.12.8.206:35474[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-02T06:21:09.403123Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m203.12.8.206:35474[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Failed to connect [3merror[0m[2m=[0mfailed to fill whole buffer
[2m2025-12-02T06:21:09.403369Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m86.248.46.31:6881[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connecting to peer
[2m2025-12-02T06:21:09.917451Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m86.248.46.31:6881[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connected [3mremote_id[0m[2m=[0m-TR2930-0hc2sqt9mid4
[2m2025-12-02T06:21:09.917536Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m86.248.46.31:6881[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Connected, requesting file
[2m2025-12-02T06:21:16.865306Z[0m [34mDEBUG[0m [1mrequest_complete_file[0m[1m{[0m[3mpeer_addr[0m[2m=[0m86.248.46.31:6881[1m}[0m[2m:[0m [2mbt_client[0m[2m:[0m Ready to download
[2m2025-12-02T06:21:16.865451Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Downloading file [3mfile_size[0m[2m=[0m702545920 [3mpiece_count[0m[2m=[0m2680 [3mpeer_address[0m[2m=[0m86.248.46.31:6881 [3mremote_id[0m[2m=[0m-TR2930-0hc2sqt9mid4
[2m2025-12-02T06:21:17.899175Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m0 [3mduration_ms[0m[2m=[0m1032
[2m2025-12-02T06:21:17.971705Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m1 [3mduration_ms[0m[2m=[0m72
[2m2025-12-02T06:21:18.542541Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2 [3mduration_ms[0m[2m=[0m570
[2m2025-12-02T06:21:18.576846Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m3 [3mduration_ms[0m[2m=[0m34
[2m2025-12-02T06:21:18.585826Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m4 [3mduration_ms[0m[2m=[0m8
[2m2025-12-02T06:21:18.622275Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m5 [3mduration_ms[0m[2m=[0m36
[2m2025-12-02T06:21:18.630372Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m6 [3mduration_ms[0m[2m=[0m8
[2m2025-12-02T06:21:18.901433Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m7 [3mduration_ms[0m[2m=[0m271
[2m2025-12-02T06:21:18.914199Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m8 [3mduration_ms[0m[2m=[0m12
[2m2025-12-02T06:21:18.924021Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m9 [3mduration_ms[0m[2m=[0m9
[2m2025-12-02T06:21:18.932650Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m10 [3mduration_ms[0m[2m=[0m8

[2;3m......< skipped rest of events >........[0m

[2m2025-12-02T06:23:41.616984Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2676 [3mduration_ms[0m[2m=[0m6
[2m2025-12-02T06:23:41.623190Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2677 [3mduration_ms[0m[2m=[0m6
[2m2025-12-02T06:23:41.629119Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2678 [3mduration_ms[0m[2m=[0m5
[2m2025-12-02T06:23:42.082875Z[0m [34mDEBUG[0m [2mbt_client::downloader::file_downloader[0m[2m:[0m Downloaded piece [3mpiece_index[0m[2m=[0m2679 [3mduration_ms[0m[2m=[0m453
[2m2025-12-02T06:23:42.082961Z[0m [32m INFO[0m [2mmain[0m[2m:[0m Received entire file [3mfile_bytes[0m[2m=[0m"455208000000909000000000000000000000000000000000000000000000000033edfa8ed5bc007cfbfc6631db6631c96653665106578edd8ec552be007cbf0006b90001f3a5ea4b06000052b441bbaa5531c930f6f9cd13721681fb55aa751083e101740b66c706f306b442eb15eb0231c95a51b408cd135b0fb6c6405083e1" [3mfile_size[0m[2m=[0m702545920 [3mdownload_duration[0m[2m=[0m"145.22s"
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
