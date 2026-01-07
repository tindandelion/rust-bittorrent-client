use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

pub fn main() {
    let mut terminal = ratatui::init();

    loop {
        terminal
            .draw(|frame| {
                let text = Line::from("Hello, world!").bold().italic();
                let widget = Paragraph::new(text).centered().block(Block::bordered());
                frame.render_widget(widget, frame.area());
            })
            .expect("failed to draw frame");

        match event::read().expect("failed to read event") {
            Event::Key(key) => {
                if key.code == KeyCode::Esc {
                    break;
                }
            }
            _ => (),
        }
    }

    ratatui::restore();
}
