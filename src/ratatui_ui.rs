use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub enum AppEvent {
    Exit,
    Probing(String),
    Downloading(usize, usize),
    Completed,
}

pub struct AppUi {
    app_state: AppState,
    event_sender: Sender<AppEvent>,
    event_receiver: Receiver<AppEvent>,
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

impl AppUi {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel::<AppEvent>();
        Self {
            app_state: AppState::default(),
            event_sender,
            event_receiver,
        }
    }

    pub fn clone_sender(&self) -> Sender<AppEvent> {
        self.event_sender.clone()
    }

    pub fn run(&mut self) -> Result<()> {
        let terminal = ratatui::init();
        Self::listen_for_keyboard_events(self.clone_sender());
        let result = self.render_loop(terminal);
        ratatui::restore();
        result
    }

    fn render_loop(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render_ui(frame))?;
            if !self.process_app_event()? {
                break Ok(());
            }
        }
    }

    fn process_app_event(&mut self) -> Result<bool> {
        match self.event_receiver.recv()? {
            AppEvent::Probing(ip_address) => {
                self.app_state.download_state = DownloadState::Probing(ip_address);
                Ok(true)
            }
            AppEvent::Downloading(current, total) => {
                self.app_state.download_state = DownloadState::Downloading(current, total);
                Ok(true)
            }
            AppEvent::Completed => Ok(false),
            AppEvent::Exit => Ok(false),
        }
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let status_text = match &self.app_state.download_state {
            DownloadState::Idle => "Idle".to_string(),
            DownloadState::Probing(ip_address) => format!("Probing: {}", ip_address),
            DownloadState::Downloading(downloaded, total) => {
                format!("Downloading: {} / {}", downloaded, total)
            }
        };

        let block = ratatui::widgets::Block::default().title(status_text);
        f.render_widget(block, f.area());
    }

    fn listen_for_keyboard_events(sender: Sender<AppEvent>) {
        thread::spawn(move || {
            loop {
                if let Event::Key(key) = event::read().unwrap() {
                    match key.code {
                        event::KeyCode::Esc => sender.send(AppEvent::Exit).unwrap(),
                        _ => {}
                    }
                }
            }
        });
    }
}
