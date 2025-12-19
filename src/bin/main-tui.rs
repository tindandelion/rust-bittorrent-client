use std::{sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{AppEvent, AppUi};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn main() -> Result<()> {
    let mut ui = AppUi::new();

    download_file(ui.clone_sender());
    ui.run()
}

fn download_file(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"];
        for ip_address in ip_addresses {
            tx.send(AppEvent::Probing(ip_address.to_string())).unwrap();
            thread::sleep(Duration::from_secs(3));
        }

        for i in 0..100 {
            tx.send(AppEvent::Downloading(i, 100)).unwrap();
            thread::sleep(Duration::from_millis(100));
        }

        tx.send(AppEvent::Completed).unwrap();
    });
}
