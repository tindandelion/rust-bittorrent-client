---
layout: post
title:  "Ratatui UI: Connecting the core"
date: 2026-02-01
---

I've done most of the work in [the previous section][background-tasks-post], so connecting the code that manages UI to the main download logic has become a quite easy change. I'll just mention briefly in this section the most notable changes I've made. 

[*Version 0.1.0 on GitHub*](https://github.com/tindandelion/rust-bittorrent-client/tree/0.1.0){: .no-github-icon}

#### _Torrent::download()_ 

[`Torrent::download()][https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.0/src/lib.rs#L21]

#### _FileDownloader_

[`FileDownloader`](https://github.com/tindandelion/rust-bittorrent-client/blob/0.1.0/src/downloader/file_downloader.rs#L28)

#### New Ratatui widgets

[Gauge](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Gauge.html)

[LineGauge](https://docs.rs/ratatui/latest/ratatui/widgets/struct.LineGauge.html)

#### Where do _tracing_ logs go? 

```rust
fn setup_tracing() -> Result<()> {
    let crate_name = env!("CARGO_PKG_NAME");
    let log_filename = format!("{}.log", crate_name);

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(File::create(&log_filename)?)
        .init();

    Ok(())
}
```

# Running the application

![Main application playback]({{ site.baseurl }}/assets/images/connecting-ui/main.gif)

# Next steps 

Probing peers in parallel