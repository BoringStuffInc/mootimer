use crate::app::App;
use crate::ui::helpers::{focused_border_style, format_duration_hms};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use serde_json::Value;

pub fn draw_timers(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_timer_list(f, app, chunks[0]);
    draw_timer_details(f, app, chunks[1]);
}

fn draw_timer_list(f: &mut Frame, app: &App, area: Rect) {
    let timer_count = app.active_timers.len();

    let items: Vec<ListItem> = if app.active_timers.is_empty() {
        vec![
            ListItem::new(""),
            ListItem::new("  No active timers."),
            ListItem::new(""),
            ListItem::new("  Start a timer from the Dashboard!"),
        ]
    } else {
        app.active_timers
            .iter()
            .enumerate()
            .map(|(i, timer)| {
                let is_selected = i == app.selected_timer_index;
                let timer_id = timer
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let short_id = &timer_id[..8.min(timer_id.len())];

                let mode = timer
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("manual");
                let state = timer
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stopped");
                let elapsed = timer
                    .get("elapsed_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let task_title = timer
                    .get("task_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No task");

                let mode_icon = match mode {
                    "pomodoro" => "🍅",
                    "countdown" => "⏳",
                    _ => "⏱️",
                };

                let state_icon = match state {
                    "running" => "▶",
                    "paused" => "⏸",
                    _ => "⏹",
                };

                let mut style = Style::default();
                if is_selected {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }

                match state {
                    "running" => style = style.fg(Color::Green),
                    "paused" => style = style.fg(Color::Yellow),
                    _ => style = style.fg(Color::Gray),
                }

                let prefix = if is_selected { "→ " } else { "  " };
                let text = format!(
                    "{}{} {} {} {} ({})",
                    prefix,
                    mode_icon,
                    state_icon,
                    format_duration_hms(elapsed),
                    task_title,
                    short_id
                );

                ListItem::new(text).style(style)
            })
            .collect()
    };

    let title = format!(" ⏱️ Active Timers ({}) ", timer_count);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(" [j/k]Nav [Space]Pause [x]Stop [r]Refresh ".to_string())
        .border_style(focused_border_style(true));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_timer_details(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Timer Details ")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(timer) = app.active_timers.get(app.selected_timer_index) {
        draw_timer_detail_content(f, timer, inner);
    } else {
        let placeholder = Paragraph::new("Select a timer to view details")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(placeholder, inner);
    }
}

fn draw_timer_detail_content(f: &mut Frame, timer: &Value, area: Rect) {
    let timer_id = timer
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let profile_id = timer
        .get("profile_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mode = timer
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("manual");
    let state = timer
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("stopped");
    let elapsed = timer
        .get("elapsed_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let task_title = timer
        .get("task_title")
        .and_then(|v| v.as_str())
        .unwrap_or("No task");
    let start_time = timer
        .get("start_time")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let state_badge = match state {
        "running" => Span::styled(
            " RUNNING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        "paused" => Span::styled(
            " PAUSED ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        _ => Span::styled(
            " STOPPED ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let mode_display = match mode {
        "pomodoro" => "🍅 Pomodoro",
        "countdown" => "⏳ Countdown",
        _ => "⏱️ Manual",
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status:   ", Style::default().add_modifier(Modifier::DIM)),
            state_badge,
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Mode:     ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                mode_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Elapsed:  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                format_duration_hms(elapsed),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Task:     ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(task_title, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Started:  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(start_time, Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Timer ID: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(timer_id, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Profile:  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(profile_id, Style::default().fg(Color::DarkGray)),
        ]),
    ];

    // Add Pomodoro-specific info
    if let Some(pomo_state) = timer.get("pomodoro_state")
        && let Some(phase) = pomo_state.get("phase").and_then(|v| v.as_str())
    {
        let session = pomo_state
            .get("current_session")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        let phase_display = match phase {
            "work" => "Work",
            "short_break" => "Short Break",
            "long_break" => "Long Break",
            _ => phase,
        };

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Phase:    ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                format!("{} (Session {})", phase_display, session),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    // Add Countdown-specific info
    if let Some(target) = timer.get("target_duration").and_then(|v| v.as_u64()) {
        let remaining = target.saturating_sub(elapsed);
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Remaining:", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                format!(" {}", format_duration_hms(remaining)),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}
