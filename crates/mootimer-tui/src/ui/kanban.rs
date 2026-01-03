use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw_kanban(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let columns = [
        ("To Do", 0, Color::Red),
        ("In Progress", 1, Color::Yellow),
        ("Done", 2, Color::Green),
    ];

    for (i, (title, col_idx, color)) in columns.iter().enumerate() {
        let is_col_selected = app.selected_column_index == *col_idx;

        let tasks = app.get_kanban_tasks(*col_idx);

        let items: Vec<ListItem> = tasks
            .iter()
            .enumerate()
            .map(|(j, task)| {
                let title = task
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let is_card_selected = is_col_selected && app.selected_kanban_card_index == j;

                let style = if is_card_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(*color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(*color)
                };

                let mut lines = vec![Line::from(format!(" {} ", title))];

                if app.show_task_description
                    && let Some(desc) = task.get("description").and_then(|v| v.as_str())
                    && !desc.trim().is_empty()
                {
                    // Contrast logic for selected card (black text on colored bg)
                    let desc_style = if is_card_selected {
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::ITALIC)
                    } else {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC)
                    };

                    lines.push(Line::from(Span::styled(format!("   {}", desc), desc_style)));
                }

                ListItem::new(lines).style(style)
            })
            .collect();

        let border_style = if is_col_selected {
            Style::default().fg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block_title = format!(" {} ({}) ", title, tasks.len());

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(block_title)
                .border_style(border_style),
        );

        f.render_widget(list, chunks[i]);
    }
}
