use std::net::SocketAddr;
use std::{sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{App, AppEvent};
use bt_client::result::Result;

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

    for (index, ip_address) in ip_addresses.iter().enumerate() {
        tx.send(AppEvent::Probing {
            address: *ip_address,
            current_index: index,
            total_count: ip_addresses.len(),
        })?;
        thread::sleep(Duration::from_secs(2));
    }

    for i in 0..100 {
        tx.send(AppEvent::Downloading(i * 1024, 100 * 1024))?;
        thread::sleep(Duration::from_millis(100));
    }

    tx.send(AppEvent::Completed)?;
    Ok(())
}
