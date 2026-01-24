use std::net::SocketAddr;
use std::{sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{App, AppEvent};
use bt_client::result::{Result, StdResult};

pub fn main() -> Result<()> {
    let mut ui = App::new();
    ui.start_background_task(download_file);
    ui.run_ui_loop()
}

fn download_file(tx: &Sender<AppEvent>) -> Result<()> {
    let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"]
        .into_iter()
        .map(|s| s.parse().unwrap())
        .collect::<Vec<SocketAddr>>();

    for ip_address in ip_addresses {
        tx.send(AppEvent::Probing(ip_address))?;
        thread::sleep(Duration::from_secs(2));
    }

    failing_io()?;
    Ok(())
}

fn failing_io() -> StdResult<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to do IO operation",
    ))
}
