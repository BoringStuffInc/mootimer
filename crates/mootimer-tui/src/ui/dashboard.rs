use crate::app::{App, DashboardPane, TimerType};
use crate::ui::big_text::BigText;
use crate::ui::buttons::{Button, render_button_row};
use crate::ui::cow::Cow;
use crate::ui::helpers::{
    build_hint_line, focused_border_style, format_duration_hm, format_duration_hms,
};
use crate::ui::tomato::Tomato;
use mootimer_core::models::{ActiveTimer, PomodoroPhase, TimerMode, TimerState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

fn format_duration_ms(seconds: u64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", minutes, secs)
}

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
    let is_focused = app.focused_pane == DashboardPane::TimerConfig;
    let active_timer: Option<ActiveTimer> = app
        .timer_info
        .clone()
        .and_then(|v| serde_json::from_value(v).ok());

    let hint = build_timer_hint(&active_timer, app);
    let hint_line = build_hint_line(hint, app.focused_pane == DashboardPane::ProfileList);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⏱  Timer ")
        .title_bottom(hint_line.right_aligned())
        .border_style(focused_border_style(is_focused));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let pane_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner_area);

    let (main_area, button_area, gauge_area) = (pane_chunks[0], pane_chunks[1], pane_chunks[3]);

    let animation_type = get_animation_type(&active_timer);
    let (info_area, animation_area) = split_for_animation(main_area, animation_type.is_some());

    if let Some(anim_area) = animation_area {
        draw_timer_animation(f, app, &active_timer, anim_area, animation_type);
    }

    if let Some(timer) = &active_timer {
        draw_active_timer_info(f, app, timer, info_area, gauge_area, animation_type);
    } else {
        draw_idle_timer_info(f, app, info_area);
    }

    let buttons = build_timer_buttons(&active_timer, is_focused, app.selected_timer_button);
    let padded_button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .split(button_area)[1];
    render_button_row(f, padded_button_area, &buttons, 1);
}

fn build_timer_hint(active_timer: &Option<ActiveTimer>, app: &App) -> &'static str {
    if let Some(timer) = active_timer {
        match timer.state {
            TimerState::Running | TimerState::Paused => " [←→]Buttons [Enter]Action [r]Refresh ",
            _ => " [t]Type [Space/Enter]Start ",
        }
    } else {
        match app.selected_timer_type {
            TimerType::Manual => " [t]Type [Space/Enter]Start ",
            _ => " [t]Type [↑↓]Dur [Space/Enter]Start ",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum AnimationType {
    Tomato,
    Cow,
    ManualBigText,
}

fn get_animation_type(active_timer: &Option<ActiveTimer>) -> Option<AnimationType> {
    let timer = active_timer.as_ref()?;
    if !timer.is_running() && !timer.is_paused() {
        return None;
    }
    match timer.mode {
        _ if timer.is_pomodoro() => Some(AnimationType::Tomato),
        TimerMode::Countdown => Some(AnimationType::Cow),
        TimerMode::Manual => Some(AnimationType::ManualBigText),
        _ => None,
    }
}

fn split_for_animation(main_area: Rect, show_animation: bool) -> (Rect, Option<Rect>) {
    if show_animation {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(32),
                Constraint::Fill(1),
            ])
            .split(main_area);
        (chunks[0], Some(chunks[1]))
    } else {
        (main_area, None)
    }
}

fn draw_timer_animation(
    f: &mut Frame,
    app: &mut App,
    active_timer: &Option<ActiveTimer>,
    area: Rect,
    animation_type: Option<AnimationType>,
) {
    match animation_type {
        Some(AnimationType::Tomato) => {
            f.render_stateful_widget(Tomato, area, &mut app.tomato_state);
        }
        Some(AnimationType::Cow) => {
            f.render_stateful_widget(Cow, area, &mut app.cow_state);
        }
        Some(AnimationType::ManualBigText) => {
            if let Some(timer) = active_timer {
                let time_str = format_duration_hms(timer.current_elapsed());
                let (text_width, text_height) = (30, 5);
                let x = area.x + (area.width.saturating_sub(text_width)) / 2;
                let y = area.y + (area.height.saturating_sub(text_height)) / 2;
                let centered_area = Rect::new(x, y, text_width, text_height);
                f.render_widget(
                    BigText::new(&time_str).style(Style::default().fg(Color::Green)),
                    centered_area,
                );
            }
        }
        None => {}
    }
}

fn draw_active_timer_info(
    f: &mut Frame,
    app: &App,
    timer: &ActiveTimer,
    info_area: Rect,
    gauge_area: Rect,
    animation_type: Option<AnimationType>,
) {
    let color = match timer.state {
        TimerState::Running => Color::Green,
        TimerState::Paused => Color::Yellow,
        _ => Color::Gray,
    };

    let state_icon = match timer.state {
        TimerState::Running => "▶",
        TimerState::Paused => "⏸",
        _ => "⏹",
    };

    let (time_display, ratio, phase_info, next_phase_info) =
        build_timer_display_info(timer, state_icon, animation_type);

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

    let state_badge = build_state_badge(timer.state);
    let mut text_lines = vec![
        Line::from(vec![
            Span::styled("STATUS: ", Style::default().add_modifier(Modifier::DIM)),
            state_badge,
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("PHASE:  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(phase_info, Style::default().add_modifier(Modifier::BOLD)),
        ]),
    ];

    if let Some(next) = next_phase_info {
        text_lines.push(Line::from(vec![
            Span::styled("NEXT:   ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(next, Style::default().fg(Color::DarkGray)),
        ]));
    }

    text_lines.push(Line::from(vec![
        Span::styled("TASK:   ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled(task_name, Style::default().fg(Color::Cyan)),
    ]));

    if !time_display.is_empty() {
        text_lines.insert(0, Line::from(""));
        text_lines.insert(
            0,
            Line::from(Span::styled(
                time_display,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
        );
    }

    let text_widget = Paragraph::new(text_lines)
        .block(Block::default().padding(ratatui::widgets::Padding::new(2, 2, 1, 1)));
    f.render_widget(text_widget, info_area);

    if let Some(r) = ratio {
        let gauge = Gauge::default()
            .block(Block::default().padding(ratatui::widgets::Padding::horizontal(2)))
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .ratio(r)
            .label(format!("{:.0}%", r * 100.0));
        f.render_widget(gauge, gauge_area);
    }
}

fn build_timer_display_info(
    timer: &ActiveTimer,
    state_icon: &str,
    animation_type: Option<AnimationType>,
) -> (String, Option<f64>, String, Option<String>) {
    if timer.is_pomodoro() {
        let remaining = timer.remaining_seconds().unwrap_or(0);
        let elapsed_in_phase = timer.current_phase_elapsed();
        let phase_duration = remaining + elapsed_in_phase;

        let ratio = if phase_duration > 0 {
            (elapsed_in_phase as f64 / phase_duration as f64).min(1.0)
        } else {
            0.0
        };

        let Some(pomo) = timer.pomodoro_state.as_ref() else {
            return (String::new(), None, "Unknown".to_string(), None);
        };

        let phase_name = match pomo.phase {
            PomodoroPhase::Work => "Work",
            PomodoroPhase::ShortBreak => "Short Break",
            PomodoroPhase::LongBreak => "Long Break",
        };

        let next_phase = match pomo.phase {
            PomodoroPhase::Work => {
                if pomo.current_session % pomo.config.sessions_until_long_break == 0 {
                    "Long Break"
                } else {
                    "Short Break"
                }
            }
            _ => "Work",
        };

        (
            format!("{} {}", state_icon, format_duration_ms(remaining)),
            Some(ratio),
            format!("{} (Session {})", phase_name, pomo.current_session),
            Some(next_phase.to_string()),
        )
    } else if timer.mode == TimerMode::Countdown {
        let elapsed = timer.current_elapsed();
        let target = timer.target_duration.unwrap_or(0);
        let remaining = target.saturating_sub(elapsed);
        let ratio = if target > 0 {
            (elapsed as f64 / target as f64).min(1.0)
        } else {
            0.0
        };

        (
            format!("{} {}", state_icon, format_duration_hms(remaining)),
            Some(ratio),
            "Countdown".to_string(),
            None,
        )
    } else {
        let time_display = if animation_type == Some(AnimationType::ManualBigText) {
            String::new()
        } else {
            format!(
                "{} {}",
                state_icon,
                format_duration_hms(timer.current_elapsed())
            )
        };
        (time_display, None, "Manual Timer".to_string(), None)
    }
}

fn build_state_badge(state: TimerState) -> Span<'static> {
    match state {
        TimerState::Running => Span::styled(
            " RUNNING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        TimerState::Paused => Span::styled(
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
    }
}

fn draw_idle_timer_info(f: &mut Frame, app: &App, info_area: Rect) {
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

fn build_timer_buttons(
    active_timer: &Option<ActiveTimer>,
    is_focused: bool,
    selected_button: usize,
) -> Vec<Button<'static>> {
    match active_timer {
        Some(timer) => match timer.state {
            TimerState::Running => vec![
                Button::new("Pause", "Space", is_focused && selected_button == 0),
                Button::new("Stop", "x", is_focused && selected_button == 1),
            ],
            TimerState::Paused => vec![
                Button::new("Resume", "Space", is_focused && selected_button == 0),
                Button::new("Stop", "x", is_focused && selected_button == 1),
            ],
            _ => vec![Button::new("Start", "Space", is_focused)],
        },
        None => vec![Button::new("Start", "Space", is_focused)],
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
                    "done" | "completed" => "✓",
                    _ => "○",
                };

                let is_selected = i == app.selected_task_index;
                let mut style = Style::default();

                match status {
                    "done" | "completed" => {
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

    let bottom_hint_line =
        build_hint_line(&bottom_hint, app.focused_pane == DashboardPane::TimerConfig);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(bottom_hint_line.right_aligned())
        .border_style(focused_border_style(
            app.focused_pane == DashboardPane::TasksList,
        ));

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

    let title = format!(" 👤 Profiles ({}) ", app.profiles.len());
    let bottom_hint = " [Enter]Switch [n]New [d]Del [r]Rename ";
    let bottom_hint_line =
        build_hint_line(bottom_hint, app.focused_pane == DashboardPane::TasksList);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(bottom_hint_line.right_aligned())
        .border_style(focused_border_style(
            app.focused_pane == DashboardPane::ProfileList,
        ));

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
                format!("⏱ {}", format_duration_hm(total_secs)),
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
