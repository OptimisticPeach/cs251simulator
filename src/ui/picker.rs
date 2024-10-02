use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style, Styled},
    symbols::border,
    text::Text,
    widgets::{Block, Paragraph, Widget},
};

use crate::util::center;

#[derive(Clone, Copy)]
pub struct Picker {
    name: char,
}

impl Picker {
    pub fn new(name: char) -> Self {
        Self { name }
    }
}

impl Widget for Picker {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_set(border::DOUBLE)
            .set_style(Style::reset().fg(Color::Cyan));

        let text = Text::raw(format!("Select this window: {}", self.name));

        let width = text.width() as u16 + 2;
        let height = text.height() as u16 + 2;

        let para = Paragraph::new(text).block(block);

        let inner = center(area, Constraint::Length(width), Constraint::Length(height));

        para.render(inner, buf);
    }
}
