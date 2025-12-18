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

pub fn main() -> Result<()> {
    let terminal = ratatui::init();

    let (tx, rx) = mpsc::channel::<AppEvent>();
    listen_for_keyboard_events(tx);

    let result = run(terminal, rx);
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

fn run(mut terminal: DefaultTerminal, rx: Receiver<AppEvent>) -> Result<()> {
    loop {
        terminal.draw(render_ui)?;
        match rx.recv()? {
            AppEvent::Exit => break Ok(()),
            _ => {}
        }
    }
}

fn render_ui(f: &mut Frame) {
    let block = ratatui::widgets::Block::default().title("Hello, world!");
    f.render_widget(block, f.area());
}
