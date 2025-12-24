use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
    style::{Color, Stylize},
    text::{Line, ToLine},
    widgets::{Block, Padding, Paragraph},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub enum AppEvent {
    Exit,
    Resize,
    Probing(String),
    Downloading(usize, usize),
    Completed,
}

pub struct AppUi {
    app_state: DownloadState,
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

impl AppUi {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel::<AppEvent>();
        Self {
            app_state: DownloadState::default(),
            event_sender,
            event_receiver,
        }
    }

    pub fn clone_sender(&self) -> Sender<AppEvent> {
        self.event_sender.clone()
    }

    pub fn run(&mut self) -> Result<()> {
        let terminal = ratatui::init();
        listen_for_keyboard_events(self.clone_sender());
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
                self.app_state = DownloadState::Probing(ip_address);
                Ok(true)
            }
            AppEvent::Downloading(current, total) => {
                self.app_state = DownloadState::Downloading(current, total);
                Ok(true)
            }
            AppEvent::Resize => Ok(true),
            AppEvent::Completed => Ok(false),
            AppEvent::Exit => Ok(false),
        }
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let status_line = match &self.app_state {
            DownloadState::Idle => Line::from("Idle").fg(Color::Green),
            DownloadState::Probing(ip_address) => {
                Line::from(format!("Probing: {}", ip_address)).fg(Color::Red)
            }
            DownloadState::Downloading(downloaded, total) => {
                Line::from(format!("Downloading: {} / {}", downloaded, total)).fg(Color::Yellow)
            }
        };

        let status = Paragraph::new(status_line).block(
            Block::bordered()
                .title(" BitTorrent Client ".to_line().bold().centered())
                .title_bottom(
                    Line::from(vec![" Press ".into(), "<ESC>".bold(), " to exit ".into()])
                        .centered(),
                )
                .padding(Padding::horizontal(1)),
        );

        f.render_widget(status, f.area());
    }
}

fn listen_for_keyboard_events(sender: Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            match event::read().unwrap() {
                Event::Key(key) => match key.code {
                    event::KeyCode::Esc => sender.send(AppEvent::Exit).unwrap(),
                    _ => {}
                },
                Event::Resize(_, _) => {
                    sender.send(AppEvent::Resize).unwrap();
                }
                _ => (),
            }
        }
    });
}
