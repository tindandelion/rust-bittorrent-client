use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ratatui::{
    Frame,
    crossterm::event::{self, Event},
    style::{Color, Stylize},
    text::{Line, ToLine},
    widgets::{Block, Padding, Paragraph},
};

use crate::result::{GenericError, Result};

pub enum AppEvent {
    Exit,
    Error(GenericError),
    Resize,
    Probing(String),
    Downloading(usize, usize),
    Completed,
}

pub struct App {
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

#[allow(clippy::new_without_default)]
impl App {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel::<AppEvent>();
        Self {
            app_state: DownloadState::default(),
            event_sender,
            event_receiver,
        }
    }

    pub fn start_background_task<F>(&self, task: F) -> thread::JoinHandle<()>
    where
        F: FnOnce(&Sender<AppEvent>) -> Result<()>,
        F: Send + 'static,
    {
        let event_sender = self.event_sender.clone();
        thread::spawn(move || {
            if let Err(err) = task(&event_sender) {
                let error_msg = format!("failed to send error event: {:?}", err);
                event_sender.send(AppEvent::Error(err)).expect(&error_msg);
            }
        })
    }

    pub fn run_ui_loop(&mut self) -> Result<()> {
        ratatui::run(|terminal| {
            self.start_background_task(listen_for_keyboard_events);
            loop {
                terminal.draw(|frame| self.render_ui(frame))?;
                if !self.process_app_event()? {
                    break Ok(());
                }
            }
        })
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
            AppEvent::Error(err) => Err(err),
        }
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let status_line = match &self.app_state {
            DownloadState::Idle => Line::from("Idle").fg(Color::Green),
            DownloadState::Probing(ip_address) => {
                Line::from(format!("Probing: {}", ip_address)).fg(Color::Red)
            }
            DownloadState::Downloading(downloaded, total) => {
                let downloaded = humansize::format_size(*downloaded, humansize::DECIMAL);
                let total = humansize::format_size(*total, humansize::DECIMAL);
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

fn listen_for_keyboard_events(sender: &Sender<AppEvent>) -> Result<()> {
    loop {
        match event::read()? {
            Event::Key(key) => {
                if let event::KeyCode::Esc = key.code {
                    sender.send(AppEvent::Exit)?;
                }
            }
            Event::Resize(_, _) => {
                sender.send(AppEvent::Resize)?;
            }
            _ => (),
        }
    }
}
