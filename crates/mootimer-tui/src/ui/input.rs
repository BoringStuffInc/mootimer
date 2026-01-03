use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_input_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = 60;
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, modal_area);

    // Use status_message as title (e.g. "Enter task title:")
    let title = if app.status_message.is_empty() {
        "Input"
    } else {
        &app.status_message
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));

    let input = Paragraph::new(app.input_buffer.as_str()).block(block);

    f.render_widget(input, modal_area);

    // Set cursor position
    // x + 1 (border) + length of buffer
    // y + 1 (border)
    // Ensure it doesn't go out of bounds of the box
    let cursor_x = (modal_area.x + 1 + app.input_buffer.len() as u16).min(modal_area.x + width - 2);
    let cursor_y = modal_area.y + 1;

    f.set_cursor_position((cursor_x, cursor_y));
}
