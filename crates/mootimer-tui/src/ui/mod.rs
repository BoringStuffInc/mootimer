//! UI rendering module

mod buttons;

use crate::app::{App, AppView, DashboardPane, InputMode};
use buttons::{render_button_row, Button};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Helper function to get profile name from ID
fn get_profile_name<'a>(app: &'a App, profile_id: &'a str) -> &'a str {
    app.profiles
        .iter()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(profile_id))
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(profile_id)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Button bar
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    // Title bar with navigation
    draw_title_bar(f, app, chunks[0]);

    // Main content based on current view
    match app.current_view {
        AppView::Dashboard => draw_dashboard(f, app, chunks[1]),
        AppView::Entries => draw_entries(f, app, chunks[1]),
        AppView::Reports => draw_reports(f, app, chunks[1]),
        AppView::Settings => draw_settings(f, app, chunks[1]),
        AppView::Logs => draw_logs(f, app, chunks[1]),
    }

    // Button bar
    draw_button_bar(f, app, chunks[2]);

    // Status bar
    draw_status_bar(f, app, chunks[3]);

    // Help modal overlay (drawn last, on top of everything)
    if app.show_help {
        draw_help_modal(f, app);
    }

    // Profile manager modal
    if app.input_mode == InputMode::ProfileManager {
        draw_profile_manager_modal(f, app);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let tabs = vec![
        ("1", "📊", "Dashboard", AppView::Dashboard),
        ("2", "📝", "Entries", AppView::Entries),
        ("3", "📈", "Reports", AppView::Reports),
        ("4", "⚙️", "Settings", AppView::Settings),
        ("5", "📋", "Logs", AppView::Logs),
    ];

    let mut spans = vec![
        Span::styled(
            "🐮 MooTimer ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("[{}] │ ", get_profile_name(app, &app.profile_id))),
    ];

    for (i, (key, icon, name, view)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }

        let is_active = *view == app.current_view;
        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        if is_active {
            spans.push(Span::styled(format!(" {}{} ", icon, name), style));
        } else {
            spans.push(Span::styled(format!("[{}]{}{}", key, icon, name), style));
        }
    }

    spans.push(Span::raw(" │ [q]Quit"));

    let title = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(title, area);
}

fn draw_button_bar(f: &mut Frame, app: &App, area: Rect) {
    let buttons: Vec<Button> = match app.current_view {
        AppView::Dashboard => {
            // Buttons depend on which pane is focused
            use crate::app::DashboardPane;
            match app.focused_pane {
                DashboardPane::TimerConfig => {
                    // Timer pane focused - show timer controls
                    let timer_state = app
                        .timer_info
                        .as_ref()
                        .and_then(|t| t.get("state"))
                        .and_then(|s| s.as_str());

                    match timer_state {
                        Some("running") => vec![
                            Button::new("Pause", "Space", app.selected_button_index == 0),
                            Button::new("Stop", "x", app.selected_button_index == 1),
                        ],
                        Some("paused") => vec![
                            Button::new("Resume", "Space", app.selected_button_index == 0),
                            Button::new("Stop", "x", app.selected_button_index == 1),
                        ],
                        _ => vec![
                            Button::new("Start Timer", "Space", app.selected_button_index == 0),
                            Button::new("Type", "t", app.selected_button_index == 1),
                        ],
                    }
                }
                DashboardPane::TasksList => {
                    // Tasks pane focused - show task controls
                    vec![
                        Button::new("New Task", "n", app.selected_button_index == 0),
                        Button::new("Edit Task", "e", app.selected_button_index == 1),
                        Button::new("Delete Task", "d", app.selected_button_index == 2),
                        Button::new("Start Timer", "Space", app.selected_button_index == 3),
                    ]
                }
            }
        }
        AppView::Entries => vec![
            Button::new("Today", "d", app.selected_button_index == 0),
            Button::new("This Week", "w", app.selected_button_index == 1),
            Button::new("This Month", "m", app.selected_button_index == 2),
            Button::new("Refresh", "r", app.selected_button_index == 3),
        ],
        AppView::Reports => {
            let profile_label = if app.report_profile == "all" {
                "All Profiles"
            } else {
                "Current Profile"
            };
            vec![
                Button::new("Daily Report", "d", app.selected_button_index == 0),
                Button::new("Weekly Report", "w", app.selected_button_index == 1),
                Button::new("Monthly Report", "m", app.selected_button_index == 2),
                Button::new(profile_label, "p", app.selected_button_index == 3),
                Button::new("Refresh", "r", app.selected_button_index == 4),
            ]
        }
        AppView::Settings => vec![
            // No buttons - Settings uses a form-based interface
        ],
        AppView::Logs => vec![
            Button::new("Refresh", "r", app.selected_button_index == 0),
            Button::new("Clear", "c", app.selected_button_index == 1),
        ],
    };

    render_button_row(f, area, &buttons, 1);
}

fn draw_help_modal(f: &mut Frame, _app: &App) {
    use ratatui::{
        text::{Line, Span},
        widgets::Clear,
    };

    // Create a centered modal that's 80% of screen size
    let area = f.area();
    let modal_width = (area.width as f32 * 0.85) as u16;
    let modal_height = (area.height as f32 * 0.85) as u16;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background (no border, just clear the area)
    f.render_widget(Clear, modal_area);

    let help_text = vec![
        Line::from(Span::styled(
            "  MooTimer - Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  GLOBAL",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [?]          Toggle this help"),
        Line::from("    [q]/[Esc]    Quit (or close help if open)"),
        Line::from("    [Ctrl-c]     Force quit"),
        Line::from(
            "    [1-5]        Jump to view (1=Dashboard 2=Entries 3=Reports 4=Settings 5=Logs)",
        ),
        Line::from("    [Shift+P]    Open profile manager"),
        Line::from("    [Ctrl+w]     Switch pane (Dashboard only)"),
        Line::from(""),
        Line::from(Span::styled(
            "  DASHBOARD - TIMER PANE (Focus with Ctrl-w)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [t]          Cycle timer type (Manual → Pomodoro → Countdown)"),
        Line::from("    [↑↓]/[j/k]   Adjust duration (Pomodoro/Countdown only, when idle)"),
        Line::from("    [Space]      Start timer (or pause/resume if running)"),
        Line::from("    [x]          Stop timer"),
        Line::from("    [r]          Refresh all data"),
        Line::from(""),
        Line::from(Span::styled(
            "  DASHBOARD - TASKS PANE (Focus with Ctrl-w)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [↑↓]/[j/k]   Navigate tasks"),
        Line::from("    [g]g         Jump to top task"),
        Line::from("    [G]          Jump to bottom task"),
        Line::from("    [Space]      Start timer with selected task"),
        Line::from("    [n]          New task"),
        Line::from("    [d]          Delete selected task"),
        Line::from("    [e]          Edit selected task"),
        Line::from(""),
        Line::from(Span::styled(
            "  ENTRIES",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [d]          Show today's entries"),
        Line::from("    [w]          Show this week's entries"),
        Line::from("    [m]          Show this month's entries"),
        Line::from("    [r]          Refresh entries"),
        Line::from(""),
        Line::from(Span::styled(
            "  REPORTS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [d]          Daily report"),
        Line::from("    [w]          Weekly report"),
        Line::from("    [m]          Monthly report"),
        Line::from("    [p]          Toggle all profiles vs current profile"),
        Line::from("    [r]          Refresh reports"),
        Line::from(""),
        Line::from(Span::styled(
            "  SETTINGS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [p]          Edit Pomodoro work duration"),
        Line::from("    [b]          Edit short break duration"),
        Line::from("    [l]          Edit long break duration"),
        Line::from("    [g]          Toggle auto-commit"),
        Line::from("    [i]          Initialize git repository"),
        Line::from("    [a]          Toggle audio alerts (🔔 ⟷ 🔇)"),
        Line::from(""),
        Line::from(Span::styled(
            "  LOGS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [r]          Refresh logs"),
        Line::from("    [c]          Clear logs"),
        Line::from(""),
        Line::from(Span::styled(
            "  PROFILE MANAGER (Shift+P)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [Shift+P]    Open/close profile manager"),
        Line::from("    [↑↓/j/k]     Navigate profiles"),
        Line::from("    [Enter/s]    Switch to selected profile"),
        Line::from("    [n]          New profile"),
        Line::from("    [d]          Delete selected profile"),
        Line::from("    [r]          Rename selected profile"),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [?], [q], or [Esc] to close this help",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("❓ Help")
                .border_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().bg(Color::Black));

    f.render_widget(help_paragraph, modal_area);
}

fn draw_profile_manager_modal(f: &mut Frame, app: &App) {
    use ratatui::{
        text::{Line, Span},
        widgets::Clear,
    };

    // Create modal
    let area = f.area();
    let modal_width = (area.width as f32 * 0.6) as u16;
    let modal_height = (area.height as f32 * 0.7) as u16;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    // Profile list
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

            let is_current = id == app.profile_id;
            let is_selected = i == app.selected_profile_index;

            let (prefix, style) = if is_current {
                (
                    "✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                ("▶ ", Style::default().fg(Color::Yellow))
            } else {
                ("  ", Style::default())
            };

            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    let profile_list = List::new(profile_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Profile Manager - [Shift+P] to close ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(profile_list, modal_area);

    // Help footer inside modal
    let help_area = Rect {
        x: modal_area.x + 2,
        y: modal_area.y + modal_area.height - 3,
        width: modal_area.width - 4,
        height: 1,
    };

    let help_text = Paragraph::new(Line::from(vec![
        Span::styled("[↑↓/j/k] ", Style::default().fg(Color::Yellow)),
        Span::raw("Navigate  "),
        Span::styled("[Enter/s] ", Style::default().fg(Color::Yellow)),
        Span::raw("Switch  "),
        Span::styled("[n] ", Style::default().fg(Color::Yellow)),
        Span::raw("New  "),
        Span::styled("[d] ", Style::default().fg(Color::Yellow)),
        Span::raw("Delete  "),
        Span::styled("[r] ", Style::default().fg(Color::Yellow)),
        Span::raw("Rename"),
    ]));
    f.render_widget(help_text, help_area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if app.input_mode != InputMode::Normal
        && app.input_mode != InputMode::ProfileManager
    {
        format!(
            "[INPUT] {} {} [Enter]Submit [Esc]Cancel",
            app.status_message, app.input_buffer
        )
    } else if !app.status_message.is_empty() && app.input_mode == InputMode::Normal {
        format!("{}", app.status_message)
    } else {
        "[1-5]Views • [↑↓/j/k]Navigate • [←→/h/l]Buttons • [Shift+P]Profiles • [q]Quit".to_string()
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(status, area);
}

// Include all the draw functions from the old ui.rs
// (Dashboard, Tasks, Entries, Reports, Settings)

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    // New layout: Timer + Stats + Profile | Tasks List
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left: Timer + Stats + Profile
            Constraint::Percentage(50), // Right: Tasks List
        ])
        .split(area);

    // Left side: Timer, Stats, and Profile selector vertically stacked
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14), // Timer with configuration
            Constraint::Length(7),  // Today's Stats (compact)
            Constraint::Min(7),     // Profile selector (with management)
        ])
        .split(main_chunks[0]);

    draw_timer_with_config(f, app, left_chunks[0]);
    draw_stats(f, app, left_chunks[1]);
    draw_profile_selector(f, app, left_chunks[2]);

    // Right side: Tasks list
    draw_tasks_list(f, app, main_chunks[1]);
}

fn draw_timer_with_config(f: &mut Frame, app: &App, area: Rect) {
    let timer_text = if let Some(timer) = &app.timer_info {
        // Timer is running - show current state
        let duration = timer
            .get("elapsed_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;

        let state = timer
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let mode = timer
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("manual");

        let color = match state {
            "running" => Color::Green,
            "paused" => Color::Yellow,
            _ => Color::Gray,
        };

        let state_icon = match state {
            "running" => "▶",
            "paused" => "⏸",
            _ => "⏹",
        };

        // For countdown timers, show remaining time
        let (time_display, time_label) = if mode == "countdown" {
            let target_secs = timer
                .get("target_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let remaining_secs = target_secs.saturating_sub(duration);
            let rem_hours = remaining_secs / 3600;
            let rem_minutes = (remaining_secs % 3600) / 60;
            let rem_seconds = remaining_secs % 60;
            (
                format!(
                    "    {} {:02}:{:02}:{:02}",
                    state_icon, rem_hours, rem_minutes, rem_seconds
                ),
                "Remaining",
            )
        } else {
            (
                format!(
                    "    {} {:02}:{:02}:{:02}",
                    state_icon, hours, minutes, seconds
                ),
                "Elapsed",
            )
        };

        let task_name = timer
            .get("task_id")
            .and_then(|tid| tid.as_str())
            .and_then(|tid| {
                app.tasks
                    .iter()
                    .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                    .and_then(|t| t.get("title"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("No task");

        vec![
            Line::from(""),
            Line::from(Span::styled(
                time_display,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "    Mode: {} | Status: {} ({})",
                mode, state, time_label
            )),
            Line::from(format!("    Task: {}", task_name)),
            Line::from(""),
            Line::from("    [Space]Pause/Resume  [x]Stop  [r]Refresh"),
        ]
    } else {
        // No timer running - show configuration
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

        vec![
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
        ]
    };

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

    let timer_widget = Paragraph::new(timer_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );
    f.render_widget(timer_widget, area);
}

fn draw_tasks_list(f: &mut Frame, app: &App, area: Rect) {
    let task_items: Vec<ListItem> = if app.tasks.is_empty() {
        vec![
            ListItem::new(""),
            ListItem::new("  No tasks yet."),
            ListItem::new(""),
            ListItem::new("  Press [n] to create one!"),
        ]
    } else {
        app.tasks
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
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let text = format!(
                    "  {} {} {}",
                    status_icon,
                    if is_selected { "→" } else { " " },
                    title
                );
                ListItem::new(text).style(style)
            })
            .collect()
    };

    let title = if app.focused_pane == DashboardPane::TasksList {
        format!(
            "Tasks ({}) ⟨ FOCUSED ⟩ - [j/k]Nav [g/G]Jump [n]New [d]Del",
            app.tasks.len()
        )
    } else {
        format!(
            "Tasks ({}) - [Ctrl-w]Focus [j/k]Nav [n]New [d]Del",
            app.tasks.len()
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
    let profile_name = get_profile_name(app, &app.profile_id);

    let active_icon = "✓";

    // Build compact profile list
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {} ", active_icon),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                profile_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (active)", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    // Show hint based on available space
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("[Shift+P] ", Style::default().fg(Color::Yellow)),
        Span::styled("Manage profiles", Style::default().fg(Color::Gray)),
    ]));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "👤 Profile ({}/{})",
                app.profiles
                    .iter()
                    .position(|p| p.get("id").and_then(|v| v.as_str()) == Some(&app.profile_id))
                    .map(|i| i + 1)
                    .unwrap_or(1),
                app.profiles.len()
            ))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(widget, area);
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

fn draw_entries(f: &mut Frame, app: &App, area: Rect) {
    let entry_items: Vec<ListItem> = if app.entries.is_empty() {
        vec![
            ListItem::new(""),
            ListItem::new("  No entries for selected period."),
            ListItem::new(""),
            ListItem::new("  Use buttons above to filter by time period."),
        ]
    } else {
        app.entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                // Get entry ID for that professional look
                let entry_id = entry
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|id| &id[..8])
                    .unwrap_or("????????");

                let duration_secs = entry
                    .get("duration_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let hours = duration_secs / 3600;
                let minutes = (duration_secs % 3600) / 60;

                // Look up task name from task_id
                let task_id = entry.get("task_id").and_then(|v| v.as_str());

                let task_display = if let Some(tid) = task_id {
                    let task_name = app
                        .tasks
                        .iter()
                        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                        .and_then(|t| t.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown task");

                    // Show task name with UUID for that enterprise feel
                    format!("{} [{}]", task_name, &tid[..8])
                } else {
                    "No task".to_string()
                };

                let mode = entry
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("manual");

                let mode_icon = match mode {
                    "pomodoro" => "🍅",
                    "countdown" => "⏲ ",
                    _ => "⏱ ",
                };
                let time_str = if hours > 0 {
                    format!("{}h {:02}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                };

                let style = if i == app.selected_entry_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let text = format!(
                    "  {} {} {} │ {} │ {}",
                    mode_icon,
                    if i == app.selected_entry_index {
                        "→"
                    } else {
                        " "
                    },
                    entry_id,
                    time_str,
                    task_display
                );
                ListItem::new(text).style(style)
            })
            .collect()
    };

    let title = format!("📝 Time Entries ({})", app.entries.len());
    let entries_list =
        List::new(entry_items).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(entries_list, area);
}

fn draw_reports(f: &mut Frame, app: &App, area: Rect) {
    // Split into summary and breakdown sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Summary stats
            Constraint::Min(5),     // Task breakdown
        ])
        .split(area);

    // Draw summary
    draw_report_summary(f, app, chunks[0]);

    // Draw task breakdown
    draw_task_breakdown(f, app, chunks[1]);
}

fn draw_report_summary(f: &mut Frame, app: &App, area: Rect) {
    let stats = match app.report_period.as_str() {
        "week" => &app.stats_week,
        "month" => &app.stats_month,
        _ => &app.stats_today,
    };

    let report_text = if let Some(stats) = stats {
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
        let manual = stats
            .get("manual_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let entries = stats
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let avg_secs = stats
            .get("avg_duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let avg_mins = avg_secs / 60;

        let profile_label = if app.report_profile == "all" {
            "All Profiles".to_string()
        } else {
            get_profile_name(app, &app.report_profile).to_string()
        };

        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "  📊 {} Summary - {}",
                    app.report_period.to_uppercase(),
                    profile_label
                ),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("  Total Time:        {}h {:02}m", hours, minutes)),
            Line::from(format!(
                "  Total Sessions:    {}  (🍅 {} pomodoro, ⏱ {} manual)",
                entries, pomodoros, manual
            )),
            Line::from(format!("  Average Session:   {}m", avg_mins)),
        ]
    } else {
        vec![Line::from(""), Line::from("  Loading...")]
    };

    let profile_label = if app.report_profile == "all" {
        "All Profiles"
    } else {
        get_profile_name(app, &app.report_profile)
    };

    let report =
        Paragraph::new(report_text).block(Block::default().borders(Borders::ALL).title(format!(
            "📈 {} Report - {}",
            app.report_period.to_uppercase(),
            profile_label
        )));
    f.render_widget(report, area);
}

fn draw_task_breakdown(f: &mut Frame, app: &App, area: Rect) {
    use std::collections::HashMap;

    // Build task breakdown from report_entries
    let mut task_map: HashMap<String, (u64, usize)> = HashMap::new(); // task_id -> (total_seconds, count)

    for entry in &app.report_entries {
        let duration = entry
            .get("duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let task_id = entry
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("No task")
            .to_string();

        let entry_data = task_map.entry(task_id).or_insert((0, 0));
        entry_data.0 += duration;
        entry_data.1 += 1;
    }

    // Convert to sorted vec
    let mut task_breakdown: Vec<(String, u64, usize)> = task_map
        .into_iter()
        .map(|(task_id, (secs, count))| (task_id, secs, count))
        .collect();

    // Sort by duration (descending)
    task_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

    // Build lines
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  By Task:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if task_breakdown.is_empty() {
        lines.push(Line::from("  No sessions recorded for this period"));
    } else {
        // Find task names from app.tasks
        for (task_id, total_secs, count) in task_breakdown.iter().take(10) {
            let task_display = if task_id == "No task" {
                "No task".to_string()
            } else {
                let task_name = app
                    .tasks
                    .iter()
                    .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(task_id))
                    .and_then(|t| t.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                // Show task name with UUID for enterprise vibes
                format!("{} [{}]", task_name, &task_id[..8])
            };

            let hours = total_secs / 3600;
            let minutes = (total_secs % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}h {:02}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            // Truncate task display if too long
            let display_name = if task_display.len() > 40 {
                format!("{}...", &task_display[..37])
            } else {
                task_display
            };

            lines.push(Line::from(format!(
                "  {:40} {:>8}  ({} sessions)",
                display_name, time_str, count
            )));
        }

        if task_breakdown.len() > 10 {
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "  ... and {} more tasks",
                task_breakdown.len() - 10
            )));
        }
    }

    let breakdown = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("📋 Task Breakdown"),
    );
    f.render_widget(breakdown, area);
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    // More compact layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Pomodoro settings
            Constraint::Length(10), // Sync settings
            Constraint::Length(5),  // Audio settings
            Constraint::Min(3),     // Help
        ])
        .split(area);

    // Pomodoro settings
    let pomodoro_text = if let Some(config) = &app.config {
        let work_dur = config
            .get("pomodoro")
            .and_then(|p| p.get("work_duration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1500)
            / 60;
        let short_break = config
            .get("pomodoro")
            .and_then(|p| p.get("short_break"))
            .and_then(|v| v.as_u64())
            .unwrap_or(300)
            / 60;
        let long_break = config
            .get("pomodoro")
            .and_then(|p| p.get("long_break"))
            .and_then(|v| v.as_u64())
            .unwrap_or(900)
            / 60;

        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Work Duration:     "),
                Span::styled(
                    format!("{} min", work_dur),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("   [p] Edit", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::raw("  Short Break:       "),
                Span::styled(
                    format!("{} min", short_break),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("   [b] Edit", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::raw("  Long Break:        "),
                Span::styled(
                    format!("{} min", long_break),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("   [l] Edit", Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![Line::from("  Loading...")]
    };

    let pomodoro = Paragraph::new(pomodoro_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("⏱  Pomodoro Timers"),
    );
    f.render_widget(pomodoro, chunks[0]);

    // Git sync settings
    let sync_text = if let Some(sync_status) = &app.sync_status {
        let initialized = sync_status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_changes = sync_status
            .get("has_changes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let branch = sync_status
            .get("current_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("none");

        let auto_commit = app
            .config
            .as_ref()
            .and_then(|c| c.get("sync"))
            .and_then(|s| s.get("auto_commit"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (status_icon, status_color) = if initialized {
            ("✓", Color::Green)
        } else {
            ("✗", Color::Red)
        };

        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Repository:  "),
                Span::styled(
                    format!(
                        "{} {}",
                        status_icon,
                        if initialized {
                            "Initialized"
                        } else {
                            "Not initialized"
                        }
                    ),
                    Style::default().fg(status_color),
                ),
                if !initialized {
                    Span::styled("   [i] Initialize", Style::default().fg(Color::Gray))
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                Span::raw("  Branch:      "),
                Span::styled(branch, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("  Changes:     "),
                Span::styled(
                    if has_changes { "Pending sync" } else { "Clean" },
                    if has_changes {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if auto_commit { "[✓]" } else { "[ ]" },
                    if auto_commit {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::raw(" Auto-commit on save"),
                Span::styled("   [g] Toggle", Style::default().fg(Color::Gray)),
            ]),
        ]
    } else {
        vec![Line::from("  Loading...")]
    };

    let sync = Paragraph::new(sync_text)
        .block(Block::default().borders(Borders::ALL).title("🔄 Git Sync"));
    f.render_widget(sync, chunks[1]);

    // Audio alerts settings
    let (audio_icon, audio_status, audio_color) = if app.audio_alerts_enabled {
        ("🔔", "Enabled", Color::Green)
    } else {
        ("🔇", "Disabled", Color::Gray)
    };

    let audio_settings = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Audio Alerts: "),
            Span::styled(
                format!("{} {}", audio_icon, audio_status),
                Style::default()
                    .fg(audio_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   [a] Toggle", Style::default().fg(Color::Gray)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("🔔 Notifications"),
    );
    f.render_widget(audio_settings, chunks[2]);

    // Help text - more comprehensive
    let help = Paragraph::new(vec![Line::from(
        "  Pomodoro: [p]Work [b]Short [l]Long  •  Sync: [g]Auto-commit [i]Init  •  Audio: [a]Toggle",
    )])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("⌨  Keyboard Shortcuts"),
    );
    f.render_widget(help, chunks[3]);
}

fn draw_logs(f: &mut Frame, app: &mut App, area: Rect) {
    let log_items: Vec<ListItem> = app
        .log_lines
        .iter()
        .map(|line| {
            // Color code by log level
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

            ListItem::new(Line::from(colored_line))
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
