use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw_logs(f: &mut Frame, app: &mut App, area: Rect) {
    let log_items: Vec<ListItem> = app
        .log_lines
        .iter()
        .map(|line| {
            let colored_line = if line.contains("ERROR") {
                Span::styled(line.clone(), Style::default().fg(Color::Red))
            } else if line.contains("WARN") {
                Span::styled(line.clone(), Style::default().fg(Color::Yellow))
            } else if line.contains("INFO") {
                Span::styled(line.clone(), Style::default().fg(Color::Cyan))
            } else if line.contains("DEBUG") {
                Span::styled(line.clone(), Style::default().fg(Color::Gray))
            } else {
                Span::raw(line.clone())
            };

            ListItem::new(colored_line)
        })
        .collect();

    let title = format!("📋 Daemon Logs ({} lines)", app.log_lines.len());
    let logs_list = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("→ ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_log_index));

    f.render_stateful_widget(logs_list, area, &mut state);
}
