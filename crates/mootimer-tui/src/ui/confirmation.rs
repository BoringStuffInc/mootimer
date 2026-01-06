use crate::app::{App, InputMode};
use crate::ui::helpers::centered_rect;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_confirmation_modal(f: &mut Frame, app: &App) {
    let modal_area = centered_rect(f.area(), 60, 10);

    f.render_widget(Clear, modal_area);

    let (title, message, yes_label) = if app.input_mode == InputMode::ConfirmQuit {
        (
            "Quit MooTimer?",
            "A timer is currently running. It will work in the background until the daemon is stopped. Quit?".to_string(),
            "  [Y]es, Quit    ",
        )
    } else if app.input_mode == InputMode::DeleteTaskConfirm {
        let task_name = app
            .tasks
            .get(app.selected_task_index)
            .and_then(|t| t.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("this task");
        (
            "Delete Task?",
            format!("Are you sure you want to delete \"{}\" ?", task_name),
            "  [Y]es, Delete  ",
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
            "  [Y]es, Delete  ",
        )
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                yes_label,
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

pub fn draw_break_finished_modal(f: &mut Frame) {
    let modal_area = centered_rect(f.area(), 60, 10);

    f.render_widget(Clear, modal_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "☕ Break is over! Ready to get back to work?",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [Enter/Space] Start Work  ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "  [x] Stop Timer  ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " ☕ Break Finished ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(paragraph, modal_area);
}
