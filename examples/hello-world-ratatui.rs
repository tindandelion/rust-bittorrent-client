use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

pub fn main() {
    // 1: Initialize the terminal
    let mut terminal = ratatui::init();
    // 2: Enter the render loop
    loop {
        // 2.1: Render the UI
        terminal
            .draw(|frame| {
                let text = Line::from("Hello, world!").bold().italic();
                let widget = Paragraph::new(text).centered().block(Block::bordered());
                frame.render_widget(widget, frame.area());
            })
            .expect("failed to draw frame");
        // 2.2: Wait for user input
        match event::read().expect("failed to read event") {
            Event::Key(key) => {
                if key.code == KeyCode::Esc {
                    break;
                }
            }
            _ => (),
        }
    }
    // 3: Restore the terminal
    ratatui::restore();
}
