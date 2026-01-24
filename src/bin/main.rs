use bt_client::{Torrent, ratatui_ui::App, result::Result};
use tracing::Level;

pub fn main() -> Result<()> {
    let torrent = Torrent::read_default_file()?;

    let mut ui = App::new();
    ui.start_background_task(|tx| torrent.download_ui(tx));
    ui.run_ui_loop()?;

    println!("Download completed successfully");
    Ok(())
}

fn _main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    Torrent::read_default_file()?.download()
}
