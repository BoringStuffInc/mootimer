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

    let columns = if app.show_archived {
        vec![
            ("Archived", 0, Color::Red),
            ("", 1, Color::DarkGray),
            ("", 2, Color::DarkGray),
        ]
    } else {
        vec![
            ("To Do", 0, Color::Red),
            ("In Progress", 1, Color::Yellow),
            ("Done", 2, Color::Green),
        ]
    };

    let arch_hint = if app.show_archived { "Rest" } else { "Arch" };
    let bottom_hint = format!(
        " [h/l]Col [j/k]Card [H/L]Move [a]{} [A]View [v]Desc ",
        arch_hint
    );

    for (i, (title, col_idx, color)) in columns.iter().enumerate() {
        let is_col_selected = app.selected_column_index == *col_idx;
        let tasks = app.get_kanban_tasks(*col_idx);

        let items: Vec<ListItem> = if tasks.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                " (empty)",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            tasks
                .iter()
                .enumerate()
                .map(|(j, task)| {
                    let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let is_card_selected = is_col_selected && app.selected_kanban_card_index == j;

                    let style = if is_card_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(*color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(*color)
                    };

                    let line = if is_card_selected && !app.show_archived {
                        let left_arrow = if *col_idx > 0 { "←[H] " } else { "     " };
                        let right_arrow = if *col_idx < 2 { " [L]→" } else { "     " };

                        let available_width = (chunks[i].width as usize).saturating_sub(2);
                        let title_width = title.chars().count();
                        let padding_total = available_width.saturating_sub(5 + 5 + title_width);
                        let padding_str = " ".repeat(padding_total);

                        Line::from(vec![
                            Span::raw(left_arrow),
                            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw(padding_str),
                            Span::raw(right_arrow),
                        ])
                    } else {
                        Line::from(format!(" {} ", title))
                    };

                    let mut lines = vec![line];

                    if app.show_task_description
                        && let Some(desc) = task.get("description").and_then(|v| v.as_str())
                        && !desc.trim().is_empty()
                    {
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
                .collect()
        };

        let border_style = if is_col_selected {
            Style::default().fg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block_title = format!(" {} ({}) ", title, tasks.len());

        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(block_title)
            .border_style(border_style);

        if is_col_selected {
            block = block.title_bottom(Line::from(bottom_hint.as_str()).right_aligned());
        }

        f.render_widget(List::new(items).block(block), chunks[i]);
    }
}
