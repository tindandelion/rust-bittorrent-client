use std::{
    net::SocketAddr,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{self, Event},
    layout::Rect,
    style::Stylize,
    symbols,
    text::{Line, ToLine},
    widgets::{Block, LineGauge, Padding, Paragraph, Widget},
};

use crate::result::{GenericError, Result};

pub enum AppEvent {
    Exit,
    Error(GenericError),
    Resize,
    Probing {
        address: SocketAddr,
        current_index: usize,
        total_count: usize,
    },
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
    Probing {
        address: SocketAddr,
        current_index: usize,
        total_count: usize,
    },
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
            AppEvent::Probing {
                address,
                current_index,
                total_count,
            } => {
                self.app_state = DownloadState::Probing {
                    address,
                    current_index,
                    total_count,
                };
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
        let app_block = Block::bordered()
            .title(" BitTorrent Client ".to_line().bold().centered())
            .title_bottom(
                Line::from(vec![" Press ".into(), "<ESC>".bold(), " to exit ".into()]).centered(),
            )
            .padding(Padding::horizontal(1));
        let content_area = app_block.inner(f.area());
        f.render_widget(app_block, f.area());

        match &self.app_state {
            DownloadState::Idle => {
                f.render_widget(Paragraph::new(Line::from("Idle").green()), content_area);
            }
            DownloadState::Probing {
                address,
                current_index,
                total_count,
            } => {
                f.render_widget(
                    ProbingStatusWidget::probing(*address, *current_index, *total_count),
                    content_area,
                );
            }
            DownloadState::Downloading(downloaded, total) => {
                f.render_widget(
                    DownloadingStatusWidget::new(*downloaded, *total),
                    content_area,
                );
            }
        };
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

struct ProbingStatusWidget {
    ip_address: SocketAddr,
    current_index: usize,
    total_count: usize,
}

impl ProbingStatusWidget {
    pub fn probing(ip_address: SocketAddr, current_index: usize, total_count: usize) -> Self {
        Self {
            ip_address,
            current_index,
            total_count,
        }
    }
}

impl Widget for ProbingStatusWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let label = format!("Probing {}", self.ip_address);
        let ratio = self.current_index as f64 / self.total_count as f64;

        LineGauge::default()
            .filled_symbol(symbols::line::THICK_HORIZONTAL)
            .label(label)
            .ratio(ratio)
            .magenta()
            .render(area, buf);
    }
}

struct DownloadingStatusWidget {
    downloaded: usize,
    total: usize,
}

impl DownloadingStatusWidget {
    pub fn new(downloaded: usize, total: usize) -> Self {
        Self { downloaded, total }
    }
}

impl Widget for DownloadingStatusWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let downloaded = humansize::format_size(self.downloaded, humansize::DECIMAL);
        let total = humansize::format_size(self.total, humansize::DECIMAL);
        let label = format!("Downloading {} / {}", downloaded, total);
        let ratio = self.downloaded as f64 / self.total as f64;

        LineGauge::default()
            .filled_symbol(symbols::line::THICK_HORIZONTAL)
            .label(label)
            .ratio(ratio)
            .yellow()
            .render(area, buf);
    }
}
