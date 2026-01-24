use std::fs::File;

use bt_client::{Torrent, ratatui_ui::App, result::Result};
use tracing::Level;

pub fn main() -> Result<()> {
    setup_tracing()?;

    let mut ui = App::new();
    ui.start_background_task(|tx| Torrent::read_default_file()?.download(tx));
    ui.run_ui_loop()?;

    println!("Download completed successfully");
    Ok(())
}

fn setup_tracing() -> Result<()> {
    let crate_name = env!("CARGO_PKG_NAME");
    let log_filename = format!("{}.log", crate_name);

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(File::create(&log_filename)?)
        .init();

    Ok(())
}
