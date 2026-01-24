use std::{sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{App, AppEvent, AppResult};

pub fn main() -> AppResult<()> {
    let mut ui = App::new();
    ui.start_background_task(download_file);
    ui.run_ui_loop()
}

fn download_file(tx: &Sender<AppEvent>) -> AppResult<()> {
    let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"];
    for ip_address in ip_addresses {
        tx.send(AppEvent::Probing(ip_address.to_string()))?;
        thread::sleep(Duration::from_secs(2));
    }

    for i in 0..100 {
        tx.send(AppEvent::Downloading(i, 100))?;
        thread::sleep(Duration::from_millis(100));
    }

    tx.send(AppEvent::Completed)?;
    Ok(())
}
