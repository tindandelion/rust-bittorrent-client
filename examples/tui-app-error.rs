use std::{result::Result, sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{App, AppEvent};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = App::new();
    ui.start_background_task(|tx| {
        if let Err(_) = download_file(&tx) {
            tx.send(AppEvent::Error).unwrap();
        }
    });
    ui.run_ui_loop()
}

fn download_file(tx: &Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"];
    for ip_address in ip_addresses {
        tx.send(AppEvent::Probing(ip_address.to_string())).unwrap();
        thread::sleep(Duration::from_secs(2));
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to download file",
    )))
}
