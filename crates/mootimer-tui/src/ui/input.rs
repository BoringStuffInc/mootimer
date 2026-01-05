use crate::app::{App, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_input_modal(f: &mut Frame, app: &App) {
    let area = f.area();

    let is_dual_field =
        app.input_mode == InputMode::NewTask || app.input_mode == InputMode::EditTask;
    let is_quick_add = app.input_mode == InputMode::QuickAddTask;

    let width = 60;
    let height = if is_dual_field { 9 } else { 5 };
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, modal_area);

    if is_dual_field {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", app.status_message))
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(modal_area);
        f.render_widget(block, modal_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(inner);

        let title_style = if app.focused_input_field == 0 {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let title_block = Block::default()
            .borders(Borders::ALL)
            .title(" Title ")
            .border_style(title_style);
        let title_input = Paragraph::new(app.input_buffer.as_str()).block(title_block);
        f.render_widget(title_input, chunks[0]);

        let desc_style = if app.focused_input_field == 1 {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let desc_block = Block::default()
            .borders(Borders::ALL)
            .title(" Description ")
            .border_style(desc_style);
        let desc_input = Paragraph::new(app.input_buffer_2.as_str()).block(desc_block);
        f.render_widget(desc_input, chunks[1]);

        let instructions = Paragraph::new(Line::from(vec![ratatui::text::Span::styled(
            " [Tab] Switch Field  [Enter] Submit  [Esc] Cancel ",
            Style::default().fg(Color::DarkGray),
        )]));
        f.render_widget(
            instructions,
            Rect::new(chunks[2].x + 1, chunks[2].y, chunks[2].width, 1),
        );

        let cursor_x = if app.focused_input_field == 0 {
            (chunks[0].x + 1 + app.input_buffer.len() as u16).min(chunks[0].x + chunks[0].width - 2)
        } else {
            (chunks[1].x + 1 + app.input_buffer_2.len() as u16)
                .min(chunks[1].x + chunks[1].width - 2)
        };
        let cursor_y = if app.focused_input_field == 0 {
            chunks[0].y + 1
        } else {
            chunks[1].y + 1
        };
        f.set_cursor_position((cursor_x, cursor_y));
    } else {
        let title = if is_quick_add {
            " 🆕 Quick Add Task "
        } else if app.status_message.is_empty() {
            " Input "
        } else {
            &app.status_message
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(modal_area);
        f.render_widget(block, modal_area);

        let input = Paragraph::new(app.input_buffer.as_str());
        f.render_widget(input, Rect::new(inner.x, inner.y, inner.width, 1));

        let instructions = Paragraph::new(Line::from(vec![ratatui::text::Span::styled(
            " [Enter] Submit  [Esc] Cancel ",
            Style::default().fg(Color::DarkGray),
        )]));
        f.render_widget(
            instructions,
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );

        let cursor_x =
            (inner.x + app.input_buffer.len() as u16).min(inner.x + inner.width - 1);
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
