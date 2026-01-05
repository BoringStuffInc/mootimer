use crate::app::{App, DashboardPane};
use mootimer_core::models::{ActiveTimer, PomodoroPhase};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14),
            Constraint::Min(6),
            Constraint::Length(7),
        ])
        .split(main_chunks[0]);

    draw_timer_with_config(f, app, left_chunks[0]);
    draw_profile_selector(f, app, left_chunks[1]);
    draw_stats(f, app, left_chunks[2]);

    draw_tasks_list(f, app, main_chunks[1]);
}

fn draw_timer_with_config(f: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.focused_pane == DashboardPane::TimerConfig {
        "⏱  Timer ⟨ FOCUSED ⟩"
    } else {
        "⏱  Timer"
    };

    let border_style = if app.focused_pane == DashboardPane::TimerConfig {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let active_timer: Option<ActiveTimer> = app
        .timer_info
        .clone()
        .and_then(|v| serde_json::from_value(v).ok());

    let show_tomato = if let Some(timer) = &active_timer {
        timer.is_pomodoro() && (timer.is_running() || timer.is_paused())
    } else {
        false
    };

    let content_chunks = if show_tomato {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(32),
            ])
            .split(inner_area)
    } else {
        Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(inner_area)
    };

    let info_area = content_chunks[0];

    if show_tomato && content_chunks.len() > 1 {
        use crate::ui::tomato::Tomato;
        f.render_stateful_widget(Tomato, content_chunks[1], &mut app.tomato_state);
    }

    if let Some(timer) = &active_timer {
        let color = match timer.state {
            mootimer_core::models::TimerState::Running => Color::Green,
            mootimer_core::models::TimerState::Paused => Color::Yellow,
            _ => Color::Gray,
        };

        let state_icon = match timer.state {
            mootimer_core::models::TimerState::Running => "▶",
            mootimer_core::models::TimerState::Paused => "⏸",
            _ => "⏹",
        };

        let (time_display, _time_label, percent, phase_info) = if timer.is_pomodoro() {
            let remaining = timer.remaining_seconds().unwrap_or(0);
            let (elapsed_in_phase, phase_duration) = (timer.current_phase_elapsed(), remaining + timer.current_phase_elapsed());

            let rem_minutes = remaining / 60;
            let rem_seconds = remaining % 60;

            let ratio = if phase_duration > 0 {
                (elapsed_in_phase as f64 / phase_duration as f64).min(1.0)
            } else {
                0.0
            };

            let Some(pomo) = timer.pomodoro_state.as_ref() else {
                return;
            };
            let phase_name = match pomo.phase {
                PomodoroPhase::Work => "Work",
                PomodoroPhase::ShortBreak => "Short Break",
                PomodoroPhase::LongBreak => "Long Break",
            };

            (
                format!("    {} {:02}:{:02}", state_icon, rem_minutes, rem_seconds),
                "Remaining",
                Some(ratio),
                format!("{} (Session {})", phase_name, pomo.current_session),
            )
        } else if timer.mode == mootimer_core::models::TimerMode::Countdown {
            let elapsed = timer.current_elapsed();
            let target = timer.target_duration.unwrap_or(0);
            let remaining = target.saturating_sub(elapsed);

            let rem_hours = remaining / 3600;
            let rem_minutes = (remaining % 3600) / 60;
            let rem_seconds = remaining % 60;

            let ratio = if target > 0 {
                (elapsed as f64 / target as f64).min(1.0)
            } else {
                0.0
            };

            (
                format!(
                    "    {} {:02}:{:02}:{:02}",
                    state_icon, rem_hours, rem_minutes, rem_seconds
                ),
                "Remaining",
                Some(ratio),
                "Countdown".to_string(),
            )
        } else {
            let elapsed = timer.current_elapsed();
            let hours = elapsed / 3600;
            let minutes = (elapsed % 3600) / 60;
            let seconds = elapsed % 60;

            (
                format!(
                    "    {} {:02}:{:02}:{:02}",
                    state_icon, hours, minutes, seconds
                ),
                "Elapsed (Work)",
                None,
                "Manual Timer".to_string(),
            )
        };

        let task_name = timer
            .task_id
            .as_ref()
            .and_then(|tid| {
                app.tasks
                    .iter()
                    .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                    .and_then(|t| t.get("title"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("No task");

        let state_str = match timer.state {
            mootimer_core::models::TimerState::Running => "running",
            mootimer_core::models::TimerState::Paused => "paused",
            _ => "stopped",
        };

        let text_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                time_display,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("    Phase: {} | Status: {}", phase_info, state_str)),
            Line::from(format!("    Task: {}", task_name)),
            Line::from(""),
            Line::from("    [Space]Pause/Resume  [x]Stop  [r]Refresh"),
        ];

        let info_chunks = if percent.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(8),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(info_area)
        } else {
            Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(info_area)
        };

        let text_widget = Paragraph::new(text_lines);
        f.render_widget(text_widget, info_chunks[0]);

        if let Some(ratio) = percent
            && info_chunks.len() >= 2
        {
            let gauge = Gauge::default()
                .block(Block::default())
                .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
                .ratio(ratio)
                .label(format!("{:.0}%", ratio * 100.0));
            f.render_widget(gauge, info_chunks[1]);
        }
    } else {
        use crate::app::TimerType;

        let selected_task = app
            .tasks
            .get(app.selected_task_index)
            .and_then(|t| t.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("No task");

        let (timer_type_display, duration_text, config_hint) = match app.selected_timer_type {
            TimerType::Manual => ("▶ Manual ▶", "Until stopped".to_string(), "[t]Change Type"),
            TimerType::Pomodoro => (
                "▶ Pomodoro ▶",
                format!("{}m work", app.pomodoro_minutes),
                "[t]Type  [↑↓]Duration",
            ),
            TimerType::Countdown => (
                "▶ Countdown ▶",
                format!("{}m", app.countdown_minutes),
                "[t]Type  [↑↓]Duration",
            ),
        };

        let text_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "    Ready to Start",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("    Type: "),
                Span::styled(
                    timer_type_display,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("    Duration: "),
                Span::styled(duration_text, Style::default().fg(Color::Green)),
            ]),
            Line::from(format!("    Task: {}", selected_task)),
            Line::from(""),
            Line::from(format!("    {}  [Space/Enter]Start", config_hint)),
        ];

        let text_widget = Paragraph::new(text_lines);
        f.render_widget(text_widget, info_area);
    }
}

fn draw_tasks_list(f: &mut Frame, app: &App, area: Rect) {
    let filtered_tasks = app.get_filtered_tasks();

    let task_items: Vec<ListItem> = if filtered_tasks.is_empty() {
        if !app.task_search.is_empty() {
            vec![
                ListItem::new(""),
                ListItem::new(format!("  No tasks match: '{}'", app.task_search)),
                ListItem::new(""),
                ListItem::new("  Press [/] to search again."),
            ]
        } else {
            vec![
                ListItem::new(""),
                ListItem::new("  No tasks yet."),
                ListItem::new(""),
                ListItem::new("  Press [n] to create one!"),
            ]
        }
    } else {
        filtered_tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let title = task
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let status = task
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("todo");

                let status_icon = match status {
                    "in_progress" => "▶",
                    "completed" => "✓",
                    _ => "○",
                };

                let is_selected = i == app.selected_task_index;
                let mut style = Style::default();

                match status {
                    "completed" => {
                        style = style.fg(Color::Green);
                        if !is_selected {
                            style = style.add_modifier(Modifier::DIM);
                        }
                    }
                    "in_progress" => {
                        style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                    }
                    _ => {
                        if is_selected {
                            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                        }
                    }
                }

                if is_selected {
                    style = style.bg(Color::DarkGray);
                }

                let text = format!(
                    "  {} {} {}",
                    status_icon,
                    if is_selected { "→" } else { " " },
                    title
                );

                let mut lines = vec![Line::from(text)];

                if app.show_task_description
                    && let Some(desc) = task.get("description").and_then(|v| v.as_str())
                    && !desc.trim().is_empty()
                {
                    lines.push(Line::from(Span::styled(
                        format!("      {}", desc),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }

                ListItem::new(lines).style(style)
            })
            .collect()
    };

    let title = if app.focused_pane == DashboardPane::TasksList {
        let base_title = if app.show_archived {
            "ARCHIVED Tasks"
        } else {
            "Tasks"
        };

        if app.task_search.is_empty() {
            format!(
                "{} ({}) ⟨ FOCUSED ⟩ - [j/k]Nav [g/G]Jump [n]New [d]Del [/]Search",
                base_title,
                filtered_tasks.len()
            )
        } else {
            format!(
                "{} ({} matched '{}') ⟨ FOCUSED ⟩ - [/]Search",
                base_title,
                filtered_tasks.len(),
                app.task_search
            )
        }
    } else {
        let base_title = if app.show_archived {
            "ARCHIVED Tasks"
        } else {
            "Tasks"
        };
        format!(
            "{} ({}) - [Ctrl-w]Focus [j/k]Nav [n]New [d]Del",
            base_title,
            filtered_tasks.len()
        )
    };

    let border_style = if app.focused_pane == DashboardPane::TasksList {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tasks_list = List::new(task_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );
    f.render_widget(tasks_list, area);
}

fn draw_profile_selector(f: &mut Frame, app: &App, area: Rect) {
    let profile_items: Vec<ListItem> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let id = profile
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let name = profile
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed");

            let is_active = id == app.profile_id;
            let is_selected = i == app.selected_profile_index;

            let mut style = Style::default();

            let prefix = if is_active {
                style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
                "✓ "
            } else {
                "  "
            };

            if app.focused_pane == DashboardPane::ProfileList && is_selected {
                style = style.bg(Color::DarkGray);
                if !is_active {
                    style = style.fg(Color::Yellow);
                }
            }

            let text = format!("{}{}", prefix, name);
            ListItem::new(text).style(style)
        })
        .collect();

    let (title, border_style) = if app.focused_pane == DashboardPane::ProfileList {
        (
            format!(
                "👤 Profiles ({}) ⟨ FOCUSED ⟩ - [Enter]Switch [n]New [d]Del",
                app.profiles.len()
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            format!("👤 Profiles ({}) - [Ctrl-w]Focus", app.profiles.len()),
            Style::default().fg(Color::DarkGray),
        )
    };

    let list = List::new(profile_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_profile_index));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let stats_text = if let Some(stats) = &app.stats_today {
        let total_secs = stats
            .get("total_duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let pomodoros = stats
            .get("pomodoro_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let entries = stats
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("  ⏱ {}h {:02}m", hours, minutes),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  •  "),
                Span::styled(
                    format!("🍅 {} pomodoros", pomodoros),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw("  •  "),
                Span::styled(
                    format!("📝 {} entries", entries),
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
        ]
    } else {
        vec![Line::from(""), Line::from("  No data for today")]
    };

    let stats_widget = Paragraph::new(stats_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("📊 Today")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(stats_widget, area);
}