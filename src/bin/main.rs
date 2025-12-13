use std::error::Error;

use bt_client::Torrent;
use tracing::Level;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    Torrent::read_default_file()?.download()
}
