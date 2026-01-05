use crate::app::{App, DashboardPane};
use crate::ui::big_text::BigText;
use mootimer_core::models::{ActiveTimer, PomodoroPhase};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(7),
        ])
        .split(main_chunks[0]);

    draw_timer_with_config(f, app, left_chunks[0]);
    draw_profile_selector(f, app, left_chunks[1]);
    draw_stats(f, app, left_chunks[2]);

    draw_tasks_list(f, app, main_chunks[1]);
}

fn draw_timer_with_config(f: &mut Frame, app: &mut App, area: Rect) {
    let title = " ⏱  Timer ".to_string();

    let border_style = if app.focused_pane == DashboardPane::TimerConfig {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let active_timer: Option<ActiveTimer> = app
        .timer_info
        .clone()
        .and_then(|v| serde_json::from_value(v).ok());

    let hint = if let Some(timer) = &active_timer {
        match timer.state {
            mootimer_core::models::TimerState::Running
            | mootimer_core::models::TimerState::Paused => {
                " [Space]Pause/Resume [x]Stop [r]Refresh "
            }
            _ => " [t]Type [Space/Enter]Start ",
        }
    } else {
        match app.selected_timer_type {
            crate::app::TimerType::Manual => " [t]Type [Space/Enter]Start ",
            _ => " [t]Type [↑↓]Dur [Space/Enter]Start ",
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(hint).right_aligned())
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let pane_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(1)])
        .split(inner_area);

    let main_area = pane_chunks[0];
    let gauge_area = pane_chunks[1];

    let show_tomato = if let Some(timer) = &active_timer {
        timer.is_pomodoro() && (timer.is_running() || timer.is_paused())
    } else {
        false
    };

    let show_cow = if let Some(timer) = &active_timer {
        timer.mode == mootimer_core::models::TimerMode::Countdown
            && (timer.is_running() || timer.is_paused())
    } else {
        false
    };

    let show_manual = if let Some(timer) = &active_timer {
        timer.mode == mootimer_core::models::TimerMode::Manual
            && (timer.is_running() || timer.is_paused())
    } else {
        false
    };

    let show_animation = show_tomato || show_cow || show_manual;

    let content_chunks = if show_animation {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(32),
                Constraint::Fill(1),
            ])
            .split(main_area)
    } else {
        Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(main_area)
    };

    let info_area = content_chunks[0];

    if show_animation && content_chunks.len() >= 3 {
        let animation_area = content_chunks[1];
        if show_tomato {
            use crate::ui::tomato::Tomato;
            f.render_stateful_widget(Tomato, animation_area, &mut app.tomato_state);
        } else if show_cow {
            use crate::ui::cow::Cow;
            f.render_stateful_widget(Cow, animation_area, &mut app.cow_state);
        } else if show_manual {
            let elapsed = active_timer.as_ref().unwrap().current_elapsed();
            let hours = elapsed / 3600;
            let minutes = (elapsed % 3600) / 60;
            let seconds = elapsed % 60;
            let time_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

            let text_width = 30;
            let text_height = 5;

            let x = animation_area.x + (animation_area.width.saturating_sub(text_width)) / 2;
            let y = animation_area.y + (animation_area.height.saturating_sub(text_height)) / 2;

            let centered_area = Rect::new(x, y, text_width, text_height);
            f.render_widget(
                BigText::new(&time_str).style(Style::default().fg(Color::Green)),
                centered_area,
            );
        }
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
            let (elapsed_in_phase, phase_duration) = (
                timer.current_phase_elapsed(),
                remaining + timer.current_phase_elapsed(),
            );

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
                if show_manual {
                    String::new()
                } else {
                    format!(
                        "    {} {:02}:{:02}:{:02}",
                        state_icon, hours, minutes, seconds
                    )
                },
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

        let state_info = match timer.state {
            mootimer_core::models::TimerState::Running => Span::styled(
                " RUNNING ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            mootimer_core::models::TimerState::Paused => Span::styled(
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

        let mut text_lines = vec![
            Line::from(vec![
                Span::styled("STATUS: ", Style::default().add_modifier(Modifier::DIM)),
                state_info,
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("PHASE:  ", Style::default().add_modifier(Modifier::DIM)),
                Span::styled(phase_info, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("TASK:   ", Style::default().add_modifier(Modifier::DIM)),
                Span::styled(task_name, Style::default().fg(Color::Cyan)),
            ]),
        ];

        if !time_display.is_empty() {
            text_lines.insert(0, Line::from(""));
            text_lines.insert(
                0,
                Line::from(Span::styled(
                    time_display.trim(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
            );
        }

        let text_widget = Paragraph::new(text_lines)
            .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)));
        f.render_widget(text_widget, info_area);

        if let Some(ratio) = percent {
            let gauge = Gauge::default()
                .block(Block::default().padding(ratatui::widgets::Padding::horizontal(2)))
                .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
                .ratio(ratio)
                .label(format!("{:.0}%", ratio * 100.0));
            f.render_widget(gauge, gauge_area);
        }
    } else {
        use crate::app::TimerType;

        let selected_task = app
            .tasks
            .get(app.selected_task_index)
            .and_then(|t| t.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("No task");

        let (timer_type_display, duration_text) = match app.selected_timer_type {
            TimerType::Manual => ("Manual", "Until stopped".to_string()),
            TimerType::Pomodoro => ("Pomodoro", format!("{}m work", app.pomodoro_minutes)),
            TimerType::Countdown => ("Countdown", format!("{}m", app.countdown_minutes)),
        };

        let text_lines = vec![
            Line::from(Span::styled(
                "Ready to Start",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Type: "),
                Span::styled(
                    timer_type_display,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Duration: "),
                Span::styled(duration_text, Style::default().fg(Color::Green)),
            ]),
            Line::from(format!("Task: {}", selected_task)),
        ];

        let text_widget = Paragraph::new(text_lines)
            .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)));
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

    let base_title = if app.show_archived {
        " ARCHIVED Tasks "
    } else {
        " Tasks "
    };

    let title = format!(" {} ({}) ", base_title, filtered_tasks.len());

    let (action_hint, view_hint) = if app.show_archived {
        ("[a]Restore", "[A]Active")
    } else {
        ("[a]Archive", "[A]Archived")
    };

    let bottom_hint = format!(
        " [j/k]Nav [g/G]Jump [n]New [d]Del {} {} [/]Search ",
        action_hint, view_hint
    );

    let border_style = if app.focused_pane == DashboardPane::TasksList {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(bottom_hint).right_aligned())
        .border_style(border_style);

    let tasks_list = List::new(task_items).block(block);
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
            format!(" 👤 Profiles ({}) ", app.profiles.len()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            format!(" 👤 Profiles ({}) ", app.profiles.len()),
            Style::default().fg(Color::DarkGray),
        )
    };

    let bottom_hint = " [Enter]Switch [n]New [d]Del [r]Rename ";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(bottom_hint).right_aligned())
        .border_style(border_style);

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_profile_index));

    f.render_stateful_widget(List::new(profile_items).block(block), area, &mut state);
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

        vec![Line::from(vec![
            Span::styled(
                format!("⏱ {}h {:02}m", hours, minutes),
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
        ])]
    } else {
        vec![Line::from("No data for today")]
    };

    let stats_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 📊 Today ")
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(stats_text).alignment(Alignment::Center),
        stats_chunks[1],
    );
}
