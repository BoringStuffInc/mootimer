use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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

    let is_dragging = app.kanban_drag.is_some();
    let drag_source_col = app.kanban_drag.as_ref().map(|d| d.source_column);
    let drag_source_card = app.kanban_drag.as_ref().map(|d| d.source_card_index);
    let drag_hover_col = app.kanban_drag.as_ref().map(|d| d.current_hover_column);

    let arch_hint = if app.show_archived { "Rest" } else { "Arch" };
    let bottom_hint = if is_dragging {
        " Release mouse to drop | Drag to another column ".to_string()
    } else {
        format!(
            " [h/l]Col [j/k]Card [H/L]Move [a]{} [A]View [v]Desc | Drag cards to move ",
            arch_hint
        )
    };

    for (i, (title, col_idx, color)) in columns.iter().enumerate() {
        let is_col_selected = app.selected_column_index == *col_idx;
        let tasks = app.get_kanban_tasks(*col_idx);

        let is_drag_source_col = drag_source_col == Some(*col_idx);
        let is_drag_target_col =
            is_dragging && drag_hover_col == Some(*col_idx) && drag_source_col != Some(*col_idx);

        let items: Vec<ListItem> = if tasks.is_empty() {
            if is_drag_target_col {
                vec![ListItem::new(Line::from(Span::styled(
                    " ┌─ Drop here ─┐",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )))]
            } else {
                vec![ListItem::new(Line::from(Span::styled(
                    " (empty)",
                    Style::default().fg(Color::DarkGray),
                )))]
            }
        } else {
            let mut list_items: Vec<ListItem> = tasks
                .iter()
                .enumerate()
                .map(|(j, task)| {
                    let task_title = task
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled");
                    let is_card_selected = is_col_selected && app.selected_kanban_card_index == j;

                    let is_being_dragged =
                        is_drag_source_col && drag_source_card == Some(j) && is_dragging;

                    let style = if is_being_dragged {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM | Modifier::CROSSED_OUT)
                    } else if is_card_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(*color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(*color)
                    };

                    let line = if is_card_selected && !app.show_archived && !is_dragging {
                        let left_arrow = if *col_idx > 0 { "←[H] " } else { "     " };
                        let right_arrow = if *col_idx < 2 { " [L]→" } else { "     " };

                        let available_width = (chunks[i].width as usize).saturating_sub(2);
                        let title_width = task_title.chars().count();
                        let padding_total = available_width.saturating_sub(5 + 5 + title_width);
                        let padding_str = " ".repeat(padding_total);

                        Line::from(vec![
                            Span::raw(left_arrow),
                            Span::styled(task_title, Style::default().add_modifier(Modifier::BOLD)),
                            Span::raw(padding_str),
                            Span::raw(right_arrow),
                        ])
                    } else if is_being_dragged {
                        Line::from(format!(" [dragging] {} ", task_title))
                    } else {
                        Line::from(format!(" {} ", task_title))
                    };

                    let mut lines = vec![line];

                    if app.show_task_description
                        && let Some(desc) = task.get("description").and_then(|v| v.as_str())
                        && !desc.trim().is_empty()
                    {
                        let desc_style = if is_being_dragged {
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::DIM)
                        } else if is_card_selected {
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

            if is_drag_target_col {
                list_items.push(ListItem::new(Line::from(Span::styled(
                    " ┌─ Drop here ─┐",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))));
            }

            list_items
        };

        let border_style = if is_drag_target_col {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if is_col_selected {
            Style::default().fg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block_title = if is_drag_target_col {
            format!(" ▼ {} ({}) ▼ ", title, tasks.len())
        } else {
            format!(" {} ({}) ", title, tasks.len())
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(block_title)
            .border_style(border_style);

        if (is_col_selected && !is_dragging) || (i == 1 && is_dragging) {
            block = block.title_bottom(Line::from(bottom_hint.as_str()).right_aligned());
        }

        f.render_widget(List::new(items).block(block), chunks[i]);
    }

    if let Some(ref drag) = app.kanban_drag {
        draw_ghost_card(f, drag, area);
    }
}

fn draw_ghost_card(f: &mut Frame, drag: &crate::app::KanbanDragState, _area: Rect) {
    let ghost_width = (drag.source_task_title.chars().count() + 6).min(30) as u16;
    let ghost_height = 3;

    let ghost_x = drag.current_mouse_x.saturating_sub(ghost_width / 2);
    let ghost_y = drag.current_mouse_y.saturating_sub(1);

    let ghost_area = Rect {
        x: ghost_x,
        y: ghost_y,
        width: ghost_width,
        height: ghost_height,
    };

    f.render_widget(Clear, ghost_area);

    let title_truncated = if drag.source_task_title.chars().count() > 24 {
        format!("{}...", &drag.source_task_title[..21])
    } else {
        drag.source_task_title.clone()
    };

    let ghost = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            title_truncated,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Black)),
    );

    f.render_widget(ghost, ghost_area);
}
