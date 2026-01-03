use crate::app::{App, InputMode};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_confirmation_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = 60;
    let height = 10;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, modal_area);

    let (title, message) = if app.input_mode == InputMode::DeleteTaskConfirm {
        let task_name = app
            .tasks
            .get(app.selected_task_index)
            .and_then(|t| t.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("this task");
        (
            "Delete Task?",
            format!("Are you sure you want to delete \"{}\" ?", task_name),
        )
    } else {
        let profile_name = app
            .profiles
            .get(app.selected_profile_index)
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("this profile");
        (
            "Delete Profile?",
            format!("Are you sure you want to delete \"{}\" ?", profile_name),
        )
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [Y]es, Delete  ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "  [N]o, Cancel   ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(paragraph, modal_area);
}
