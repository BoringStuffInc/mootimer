//! Button rendering utilities

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Button<'a> {
    pub label: &'a str,
    pub shortcut: &'a str,
    pub selected: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str, shortcut: &'a str, selected: bool) -> Self {
        Self {
            label,
            shortcut,
            selected,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let style = if self.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        let text = format!(" [{}] {} ", self.shortcut, self.label);
        let button = Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(button, area);
    }
}

pub fn render_button_row(f: &mut Frame, area: Rect, buttons: &[Button], spacing: u16) {
    let button_count = buttons.len();
    if button_count == 0 {
        return;
    }

    // Calculate constraints for buttons with spacing
    let mut constraints = Vec::new();
    for i in 0..button_count {
        constraints.push(Constraint::Percentage(100 / button_count as u16));
        if i < button_count - 1 && spacing > 0 {
            constraints.push(Constraint::Length(spacing));
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let mut chunk_index = 0;
    for button in buttons {
        button.render(f, chunks[chunk_index]);
        chunk_index += 2; // Skip spacing chunks
    }
}
