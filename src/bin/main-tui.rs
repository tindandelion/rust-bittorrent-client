use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

enum AppEvent {
    Exit,
    NoOp,
}

#[derive(Default)]
enum DownloadState {
    #[default]
    Idle,
    Probing(String),
    Downloading(usize, usize),
}

#[derive(Default)]
struct AppState {
    download_state: DownloadState,
}

pub fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();
    let app_state = AppState::default();

    listen_for_keyboard_events(tx);
    let terminal = ratatui::init();
    let result = run(terminal, rx, &app_state);
    ratatui::restore();

    result
}

fn listen_for_keyboard_events(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    event::KeyCode::Esc => tx.send(AppEvent::Exit).unwrap(),
                    _ => {}
                }
            }
        }
    });
}

fn run(mut terminal: DefaultTerminal, rx: Receiver<AppEvent>, app_state: &AppState) -> Result<()> {
    loop {
        terminal.draw(|frame| render_ui(frame, app_state))?;
        match rx.recv()? {
            AppEvent::Exit => break Ok(()),
            _ => {}
        }
    }
}

fn render_ui(f: &mut Frame, app_state: &AppState) {
    let status_text = match &app_state.download_state {
        DownloadState::Idle => "Idle".to_string(),
        DownloadState::Probing(ip_address) => format!("Probing: {}", ip_address),
        DownloadState::Downloading(downloaded, total) => {
            format!("Downloading: {} / {}", downloaded, total)
        }
    };

    let block = ratatui::widgets::Block::default().title(status_text);
    f.render_widget(block, f.area());
}
