use std::{result::Result, sync::mpsc::Sender, thread, time::Duration};

use bt_client::ratatui_ui::{App, AppError, AppEvent, AppResult};

pub fn main() -> AppResult<()> {
    let mut ui = App::new();
    ui.start_background_task(download_file);
    ui.run_ui_loop()
}

fn download_file(tx: &Sender<AppEvent>) -> Result<(), AppError> {
    let ip_addresses = vec!["127.0.0.1:6881", "127.0.0.2:6882", "127.0.0.3:6883"];
    for ip_address in ip_addresses {
        tx.send(AppEvent::Probing(ip_address.to_string()))?;
        thread::sleep(Duration::from_secs(2));
    }

    failing_io()?;
    Ok(())
}

fn failing_io() -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to do IO operation",
    ))
}
