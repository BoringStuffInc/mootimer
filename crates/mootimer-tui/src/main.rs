mod app;
mod ui;

use anyhow::Result;
use app::{App, AppView, InputMode};
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};
use mootimer_client::MooTimerClient;
use ratatui::{Terminal, backend::CrosstermBackend};
use serde_json::json;
use std::io;
use tokio::time::Duration;
use tracing::info;

fn setup_logging() -> Result<()> {
    let mut log_path = std::env::temp_dir();
    log_path.push("mootimer-tui.log");

    let log_file = std::fs::File::create(log_path)?;
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_env_filter("mootimer_tui=trace")
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);

        tracing::error!(?panic_info, "Application panicked");

        eprintln!("A fatal error occurred: {}", panic_info);

        original_hook(panic_info);
    }));
}

#[derive(Parser, Debug)]
#[command(name = "mootimer")]
#[command(about = "MooTimer TUI - Professional work timer", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "/tmp/mootimer.sock")]
    socket: String,

    #[arg(short, long)]
    profile: Option<String>,
}

fn send_os_notification(title: &str, body: &str) {
    if let Err(e) = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon("clock")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show()
    {
        tracing::error!("Failed to send notification: {}", e);
    }
}

fn send_urgent_notification(title: &str, body: &str) {
    let mut notification = notify_rust::Notification::new();
    notification
        .summary(title)
        .body(body)
        .icon("alarm-clock")
        .timeout(notify_rust::Timeout::Milliseconds(10000));

    #[cfg(all(unix, not(target_os = "macos")))]
    notification.urgency(notify_rust::Urgency::Critical);

    if let Err(e) = notification.show() {
        tracing::error!("Failed to send urgent notification: {}", e);
    }
}

fn audio_alert(app: &App) {
    if app.audio_alerts_enabled {
        print!("\x07");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

async fn handle_daemon_notification(
    app: &mut App,
    notification: mootimer_client::Notification,
) -> Result<()> {
    match notification.method.as_str() {
        "timer.event" => {
            if let Some(event_type) = notification.params.get("event_type")
                && let Some(event_type_obj) = event_type.as_object()
                && let Some(type_str) = event_type_obj.get("type").and_then(|v| v.as_str())
            {
                match type_str {
                    "tick" => {
                        if let Some(remaining) = event_type_obj
                            .get("remaining_seconds")
                            .and_then(|v| v.as_u64())
                            && remaining <= 300
                            && remaining > 295
                            && !app.five_min_warning_shown
                        {
                            app.five_min_warning_shown = true;
                            app.status_message = "⚠️  5 minutes remaining!".to_string();
                            audio_alert(app);
                            send_os_notification(
                                "⏰ 5 Minutes Left",
                                "Your countdown timer is almost complete!",
                            );
                        }

                        app.refresh_timer().await?;
                    }
                    "started" => {
                        app.status_message = "Timer started".to_string();
                        app.five_min_warning_shown = false;
                        app.refresh_timer().await?;
                    }
                    "stopped" => {
                        if !app.status_message.contains("COUNTDOWN COMPLETED") {
                            app.status_message = "Timer stopped, entry saved!".to_string();
                        }
                        app.refresh_timer().await?;
                        app.refresh_stats().await?;
                        app.refresh_entries().await?;
                    }
                    "paused" => {
                        app.status_message = "Timer paused".to_string();
                        app.refresh_timer().await?;
                    }
                    "resumed" => {
                        app.status_message = "Timer resumed".to_string();
                        app.refresh_timer().await?;
                    }
                    "cancelled" => {
                        app.status_message = "Timer cancelled".to_string();
                        app.refresh_timer().await?;
                    }
                    "countdown_completed" => {
                        app.status_message = "🔔 COUNTDOWN COMPLETED! 🔔".to_string();

                        if app.cow_modal_enabled {
                            app.show_cow_modal = true;
                        }

                        audio_alert(app);

                        send_urgent_notification(
                            "⏰ Countdown Complete!",
                            "Your countdown timer has finished.",
                        );

                        app.refresh_timer().await?;
                        app.refresh_stats().await?;
                        app.refresh_entries().await?;
                    }
                    "phase_completed" => {
                        if let Some(phase_obj) = event_type_obj.get("phase") {
                            let phase = phase_obj.as_str().unwrap_or("unknown");
                            app.status_message = format!("🍅 {} phase completed!", phase);

                            audio_alert(app);

                            if phase == "short_break" || phase == "long_break" {
                                app.input_mode = InputMode::PomodoroBreakFinished;
                            }

                            let (title, body) = match phase {
                                "work" => ("🍅 Work Complete!", "Time for a break!"),
                                "short_break" => ("☕ Break Over", "Ready to focus again?"),
                                "long_break" => ("🎉 Long Break Over", "Let's get back to work!"),
                                _ => ("🍅 Pomodoro", "Phase completed"),
                            };
                            send_os_notification(title, body);
                        }
                        app.refresh_timer().await?;
                    }
                    "phase_changed" => {
                        if let Some(phase_obj) = event_type_obj.get("new_phase") {
                            let phase = phase_obj.as_str().unwrap_or("unknown");
                            app.status_message = format!("🍅 Starting {} phase", phase);

                            if phase == "short_break" || phase == "long_break" {
                                let (title, body) = if phase == "short_break" {
                                    ("☕ Short Break", "Take a quick 5-minute break!")
                                } else {
                                    ("🎉 Long Break!", "You've earned a longer break!")
                                };
                                send_os_notification(title, body);
                            }
                        }
                        app.refresh_timer().await?;
                    }
                    _ => {}
                }
            }
        }
        "task.event" => {
            if let Some(event_type) = notification.params.get("event_type")
                && let Some(event_type_obj) = event_type.as_object()
                && let Some(type_str) = event_type_obj.get("type").and_then(|v| v.as_str())
            {
                match type_str {
                    "created" => {
                        app.status_message = "Task created".to_string();
                    }
                    "updated" => {
                        app.status_message = "Task updated".to_string();
                    }
                    "deleted" => {
                        app.status_message = "Task deleted".to_string();
                    }
                    _ => {}
                }
                app.refresh_tasks().await?;
            }
        }
        "entry.event" => {
            app.refresh_entries().await?;
            app.refresh_stats().await?;
        }
        "profile.event" => {
            if let Some(event_type) = notification.params.get("event_type")
                && let Some(event_type_obj) = event_type.as_object()
                && let Some(type_str) = event_type_obj.get("type").and_then(|v| v.as_str())
            {
                match type_str {
                    "created" => {
                        app.status_message = "Profile created".to_string();
                    }
                    "updated" => {
                        app.status_message = "Profile updated".to_string();
                    }
                    "deleted" => {
                        app.status_message = "Profile deleted".to_string();
                    }
                    _ => {}
                }
                app.refresh_profiles().await?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_mouse_event(
    app: &mut App,
    mouse: event::MouseEvent,
    term_size: ratatui::layout::Rect,
) -> Result<()> {
    if mouse.kind == event::MouseEventKind::Down(event::MouseButton::Left) {
        if mouse.row == 1 {
            let tabs = [
                ("1", "📊", "Dashboard", AppView::Dashboard),
                ("2", "📋", "Kanban", AppView::Kanban),
                ("3", "📝", "Entries", AppView::Entries),
                ("4", "📈", "Reports", AppView::Reports),
                ("5", "⚙️", "Settings", AppView::Settings),
                ("6", "📋", "Logs", AppView::Logs),
            ];

            let profile_name = app.get_profile_name();
            let prefix_width = 11 + 1 + profile_name.len() as u16 + 1 + 1 + 1 + 1;

            let mut total_width = prefix_width;
            let mut tab_regions = Vec::new();

            for (i, (_key, _icon, name, view)) in tabs.iter().enumerate() {
                if i > 0 {
                    total_width += 1;
                }

                let is_active = *view == app.current_view;

                let content_width = if is_active {
                    1 + 2 + name.len() as u16 + 1
                } else {
                    1 + 1 + 1 + 2 + name.len() as u16
                };

                tab_regions.push((*view, content_width));
                total_width += content_width;
            }

            total_width += 10;

            let start_x = (term_size.width.saturating_sub(total_width)) / 2;
            let mut current_x = start_x + prefix_width;

            for (i, (view, width)) in tab_regions.iter().enumerate() {
                if i > 0 {
                    current_x += 1;
                }

                if mouse.column >= current_x && mouse.column < current_x + width {
                    let old_view = app.current_view;
                    let new_view = *view;

                    if old_view == AppView::Dashboard && new_view == AppView::Kanban {
                        let sync_target = {
                            let filtered = app.get_filtered_tasks();
                            if let Some(task) = filtered.get(app.selected_task_index)
                                && let Some(tid) = task.get("id").and_then(|v| v.as_str())
                            {
                                let status = task
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("todo");
                                let col_idx = match status {
                                    "todo" => Some(0),
                                    "in_progress" => Some(1),
                                    "done" | "completed" => Some(2),
                                    _ => None,
                                };
                                col_idx.map(|c| (c, tid.to_string()))
                            } else {
                                None
                            }
                        };

                        if let Some((c, tid)) = sync_target {
                            app.selected_column_index = c;
                            let k_tasks = app.get_kanban_tasks(c);
                            if let Some(idx) = k_tasks
                                .iter()
                                .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
                            {
                                app.selected_kanban_card_index = idx;
                            }
                        }
                    } else if old_view == AppView::Kanban && new_view == AppView::Dashboard {
                        let target_tid = {
                            let tasks = app.get_kanban_tasks(app.selected_column_index);
                            tasks
                                .get(app.selected_kanban_card_index)
                                .and_then(|task| task.get("id").and_then(|v| v.as_str()))
                                .map(|s| s.to_string())
                        };

                        if let Some(tid) = target_tid {
                            let filtered = app.get_filtered_tasks();
                            if let Some(idx) = filtered
                                .iter()
                                .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
                            {
                                app.selected_task_index = idx;
                            }
                        }
                    }

                    app.current_view = *view;
                    match view {
                        AppView::Dashboard => app.refresh_tasks().await?,
                        AppView::Kanban => app.refresh_tasks().await?,
                        AppView::Entries => app.refresh_entries().await?,
                        AppView::Reports => {
                            app.refresh_tasks().await?;
                            app.refresh_reports().await?;
                        }
                        AppView::Settings => {}
                        AppView::Logs => app.refresh_logs().await?,
                    }
                    return Ok(());
                }

                current_x += width;
            }
        } else if app.current_view == AppView::Kanban {
            handle_kanban_mouse(app, mouse, term_size).await?;
        }
    }
    Ok(())
}

async fn handle_kanban_mouse(
    app: &mut App,
    mouse: event::MouseEvent,
    term_size: ratatui::layout::Rect,
) -> Result<()> {
    if mouse.row >= 3 && mouse.row < term_size.height.saturating_sub(6) {
        let col_width = term_size.width.saturating_div(3);
        if col_width == 0 {
            return Ok(());
        }
        let col_idx = (mouse.column.saturating_div(col_width)).min(2) as usize;

        app.selected_column_index = col_idx;

        if mouse.row >= 4 {
            let item_idx = (mouse.row - 4) as usize;
            let tasks_len = app.get_kanban_tasks(col_idx).len();
            if item_idx < tasks_len {
                app.selected_kanban_card_index = item_idx;
            }
        }
    }
    Ok(())
}

async fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    if app.show_cow_modal {
        app.show_cow_modal = false;
        return Ok(());
    }

    if app.input_mode == InputMode::DeleteTaskConfirm
        || app.input_mode == InputMode::DeleteProfileConfirm
        || app.input_mode == InputMode::ConfirmQuit
        || app.input_mode == InputMode::PomodoroBreakFinished
    {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter | KeyCode::Char(' ') => {
                if app.input_mode == InputMode::ConfirmQuit {
                    app.should_quit = true;
                } else if app.input_mode == InputMode::PomodoroBreakFinished {
                    app.resume().await?;
                    app.input_mode = InputMode::Normal;
                } else if app.input_mode == InputMode::DeleteTaskConfirm {
                    app.delete_selected_task().await?;
                    app.input_mode = InputMode::Normal;
                } else {
                    app.delete_selected_profile().await?;
                    app.input_mode = InputMode::Normal;
                }
            }
            KeyCode::Char('x') if app.input_mode == InputMode::PomodoroBreakFinished => {
                app.stop_timer().await?;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') => {
                if app.input_mode != InputMode::PomodoroBreakFinished {
                    app.input_mode = InputMode::Normal;
                }
            }
            _ => {}
        }
        return Ok(());
    }

    if app.input_mode != InputMode::Normal {
        match code {
            KeyCode::Tab | KeyCode::Down | KeyCode::Up
                if app.input_mode == InputMode::NewTask
                    || app.input_mode == InputMode::EditTask =>
            {
                app.focused_input_field = if app.focused_input_field == 0 { 1 } else { 0 };
            }
            KeyCode::Enter => {
                app.submit_input().await?;
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input_buffer.clear();
                app.input_buffer_2.clear();
                app.focused_input_field = 0;
                app.temp_task_title = None;
            }
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    if c == 'c' {
                        app.should_quit = true;
                    }
                } else {
                    app.handle_input_char(c);
                }
            }
            KeyCode::Backspace => {
                app.handle_input_backspace();
            }
            _ => {}
        }
        return Ok(());
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.show_help {
                app.toggle_help();
            } else {
                let is_running = if let Some(timer) = &app.timer_info
                    && let Some(state) = timer.get("state").and_then(|v| v.as_str())
                {
                    state == "running" || state == "paused"
                } else {
                    false
                };

                if is_running {
                    app.input_mode = InputMode::ConfirmQuit;
                    print!("\x07");
                } else {
                    app.should_quit = true;
                }
            }
        }
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_view == AppView::Dashboard {
                app.focused_pane = match app.focused_pane {
                    app::DashboardPane::TimerConfig => app::DashboardPane::TasksList,
                    app::DashboardPane::TasksList => app::DashboardPane::ProfileList,
                    app::DashboardPane::ProfileList => app::DashboardPane::TimerConfig,
                };
            }
        }

        KeyCode::Char('t') => {
            if app.current_view == AppView::Dashboard
                && app.focused_pane == app::DashboardPane::TimerConfig
            {
                app.cycle_timer_type();
            }
        }
        KeyCode::Char('T') if modifiers.contains(KeyModifiers::SHIFT) => {
            if app.current_view == AppView::Dashboard
                && app.focused_pane == app::DashboardPane::TimerConfig
            {
                app.cycle_timer_type_reverse();
            }
        }
        KeyCode::Char('1') => {
            if app.current_view == AppView::Kanban {
                let target_tid = {
                    let tasks = app.get_kanban_tasks(app.selected_column_index);
                    tasks
                        .get(app.selected_kanban_card_index)
                        .and_then(|task| task.get("id").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                };

                if let Some(tid) = target_tid {
                    let filtered = app.get_filtered_tasks();
                    if let Some(idx) = filtered
                        .iter()
                        .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
                    {
                        app.selected_task_index = idx;
                    }
                }
            }
            app.current_view = AppView::Dashboard;
            app.refresh_tasks().await?;
        }
        KeyCode::Char('2') => {
            if app.current_view == AppView::Dashboard {
                let sync_target = {
                    let filtered = app.get_filtered_tasks();
                    if let Some(task) = filtered.get(app.selected_task_index)
                        && let Some(tid) = task.get("id").and_then(|v| v.as_str())
                    {
                        let status = task
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("todo");
                        let col_idx = match status {
                            "todo" => Some(0),
                            "in_progress" => Some(1),
                            "done" | "completed" => Some(2),
                            _ => None,
                        };
                        col_idx.map(|c| (c, tid.to_string()))
                    } else {
                        None
                    }
                };

                if let Some((c, tid)) = sync_target {
                    app.selected_column_index = c;
                    let k_tasks = app.get_kanban_tasks(c);
                    if let Some(idx) = k_tasks
                        .iter()
                        .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
                    {
                        app.selected_kanban_card_index = idx;
                    }
                }
            }
            app.current_view = AppView::Kanban;
            app.refresh_tasks().await?;
        }
        KeyCode::Char('3') => {
            app.current_view = AppView::Entries;
            app.refresh_entries().await?;
        }
        KeyCode::Char('4') => {
            app.current_view = AppView::Reports;
            app.refresh_tasks().await?;
            app.refresh_reports().await?;
        }
        KeyCode::Char('5') => app.current_view = AppView::Settings,
        KeyCode::Char('6') => {
            app.current_view = AppView::Logs;
            app.refresh_logs().await?;
        }

        KeyCode::PageUp => app.list_page_up(),
        KeyCode::PageDown => app.list_page_down(),

        _ => match app.current_view {
            AppView::Dashboard => handle_dashboard_keys(app, code, modifiers).await?,
            AppView::Kanban => handle_kanban_keys(app, code, modifiers).await?,
            AppView::Entries => handle_entries_keys(app, code).await?,
            AppView::Reports => handle_reports_keys(app, code).await?,
            AppView::Settings => handle_settings_keys(app, code).await?,
            AppView::Logs => handle_logs_keys(app, code).await?,
        },
    }

    Ok(())
}

async fn handle_dashboard_keys(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<()> {
    use crate::app::DashboardPane;

    let is_timer_active = if let Some(timer) = &app.timer_info {
        if let Some(state) = timer.get("state").and_then(|v| v.as_str()) {
            state == "running" || state == "paused"
        } else {
            false
        }
    } else {
        false
    };

    match app.focused_pane {
        DashboardPane::TimerConfig => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.timer_info.is_none() {
                    app.adjust_timer_duration_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.timer_info.is_none() {
                    app.adjust_timer_duration_down();
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(timer) = &app.timer_info
                    && let Some(state) = timer.get("state").and_then(|v| v.as_str())
                    && (state == "running" || state == "paused")
                {
                    app.toggle_pause().await?;
                    return Ok(());
                }
                app.start_selected_timer().await?;
            }
            KeyCode::Char('x') => {
                app.stop_timer().await?;
            }
            KeyCode::Char('r') => {
                app.status_message = "Refreshing...".to_string();
                app.refresh_all().await?;
                app.status_message = "Refreshed!".to_string();
            }
            _ => {}
        },
        DashboardPane::TasksList => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if is_timer_active {
                    app.status_message = "Cannot change task while timer is running!".to_string();
                    print!("\x07");
                } else {
                    app.list_previous();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if is_timer_active {
                    app.status_message = "Cannot change task while timer is running!".to_string();
                    print!("\x07");
                } else {
                    app.list_next();
                }
            }
            KeyCode::Char('g') => {
                if is_timer_active {
                    app.status_message = "Cannot change task while timer is running!".to_string();
                    print!("\x07");
                } else {
                    app.selected_task_index = 0;
                }
            }
            KeyCode::Char('G') => {
                if is_timer_active {
                    app.status_message = "Cannot change task while timer is running!".to_string();
                    print!("\x07");
                } else {
                    app.selected_task_index = app.tasks.len().saturating_sub(1);
                }
            }
            KeyCode::Char('n') => {
                app.input_mode = InputMode::NewTask;
                app.input_buffer.clear();
                app.input_buffer_2.clear();
                app.focused_input_field = 0;
                app.status_message = "New Task".to_string();
            }
            KeyCode::Char('v') => {
                app.show_task_description = !app.show_task_description;
                app.status_message = if app.show_task_description {
                    "Showing task descriptions".to_string()
                } else {
                    "Hidden task descriptions".to_string()
                };
            }
            KeyCode::Char('/') => {
                app.input_mode = InputMode::SearchTasks;
                app.input_buffer.clear();
                app.status_message = "Search tasks:".to_string();
            }
            KeyCode::Char('d') => {
                if !app.tasks.is_empty() {
                    app.input_mode = InputMode::DeleteTaskConfirm;
                }
            }
            KeyCode::Char('a') => {
                let filtered_tasks = app.get_filtered_tasks();
                if let Some(task) = filtered_tasks.get(app.selected_task_index)
                    && let Some(id) = task.get("id").and_then(|v| v.as_str())
                {
                    let id_owned = id.to_string();
                    app.archive_task(&id_owned).await?;
                }
            }
            KeyCode::Char('A') if modifiers.contains(KeyModifiers::SHIFT) => {
                app.show_archived = !app.show_archived;
                app.selected_task_index = 0;
                app.status_message = if app.show_archived {
                    "Viewing ARCHIVED tasks".to_string()
                } else {
                    "Viewing ACTIVE tasks".to_string()
                };
            }
            KeyCode::Char('e') => {
                app.edit_selected_task().await?;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(timer) = &app.timer_info
                    && let Some(state) = timer.get("state").and_then(|v| v.as_str())
                    && (state == "running" || state == "paused")
                {
                    app.toggle_pause().await?;
                    return Ok(());
                }
                app.start_selected_timer().await?;
            }
            KeyCode::Char('x') => {
                app.stop_timer().await?;
            }
            KeyCode::Char('r') => {
                app.status_message = "Refreshing...".to_string();
                app.refresh_all().await?;
                app.status_message = "Refreshed!".to_string();
            }
            _ => {}
        },
        DashboardPane::ProfileList => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selected_profile_index > 0 {
                    app.selected_profile_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selected_profile_index < app.profiles.len().saturating_sub(1) {
                    app.selected_profile_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('s') => {
                app.switch_to_selected_profile().await?;
            }
            KeyCode::Char('n') => {
                app.input_mode = InputMode::NewProfile;
                app.input_buffer.clear();
                app.status_message = "Enter profile name:".to_string();
            }
            KeyCode::Char('d') => {
                if !app.profiles.is_empty() {
                    app.input_mode = InputMode::DeleteProfileConfirm;
                }
            }
            KeyCode::Char('r') => {
                app.input_mode = InputMode::RenameProfile;
                app.input_buffer.clear();
                app.status_message = "Enter new profile name:".to_string();
            }
            _ => {}
        },
    }
    Ok(())
}

async fn handle_kanban_keys(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    let col_len = app.get_kanban_tasks(app.selected_column_index).len();
    if app.selected_kanban_card_index >= col_len && col_len > 0 {
        app.selected_kanban_card_index = col_len.saturating_sub(1);
    } else if col_len == 0 {
        app.selected_kanban_card_index = 0;
    }

    match code {
        KeyCode::Left | KeyCode::Char('h') => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.move_kanban_card(-1).await?;
            } else if app.selected_column_index > 0 {
                app.selected_column_index -= 1;
                app.selected_kanban_card_index = 0;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.move_kanban_card(1).await?;
            } else if app.selected_column_index < 2 {
                app.selected_column_index += 1;
                app.selected_kanban_card_index = 0;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_kanban_card_index > 0 {
                app.selected_kanban_card_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let col_len = app.get_kanban_tasks(app.selected_column_index).len();
            if col_len > 0 && app.selected_kanban_card_index < col_len.saturating_sub(1) {
                app.selected_kanban_card_index += 1;
            }
        }
        KeyCode::Char('H') => app.move_kanban_card(-1).await?,
        KeyCode::Char('L') => app.move_kanban_card(1).await?,

        KeyCode::Char('A') if modifiers.contains(KeyModifiers::SHIFT) => {
            app.show_archived = !app.show_archived;
            app.selected_column_index = 0;
            app.selected_kanban_card_index = 0;
            app.status_message = if app.show_archived {
                "Viewing ARCHIVED tasks".to_string()
            } else {
                "Viewing ACTIVE tasks".to_string()
            };
        }
        KeyCode::Char('n') => {
            app.input_mode = InputMode::NewTask;
            app.input_buffer.clear();
            app.input_buffer_2.clear();
            app.focused_input_field = 0;
            app.status_message = "New Task".to_string();
        }
        KeyCode::Char('v') => {
            app.show_task_description = !app.show_task_description;
            app.status_message = if app.show_task_description {
                "Showing task descriptions".to_string()
            } else {
                "Hidden task descriptions".to_string()
            };
        }
        KeyCode::Char('e') => {
            let title_to_edit = {
                let tasks = app.get_kanban_tasks(app.selected_column_index);
                if let Some(task) = tasks.get(app.selected_kanban_card_index) {
                    task.get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            };

            if let Some(title) = title_to_edit {
                app.input_mode = InputMode::EditTask;
                let task_id = {
                    let tasks = app.get_kanban_tasks(app.selected_column_index);
                    tasks
                        .get(app.selected_kanban_card_index)
                        .and_then(|t| t.get("id").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                };

                if let Some(tid) = task_id
                    && let Some(idx) = app
                        .tasks
                        .iter()
                        .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
                {
                    app.selected_task_index = idx;
                }

                app.input_buffer = title;
                app.status_message = "Edit task title:".to_string();
            }
        }
        KeyCode::Char('d') => {
            let task_id = {
                let tasks = app.get_kanban_tasks(app.selected_column_index);
                tasks
                    .get(app.selected_kanban_card_index)
                    .and_then(|t| t.get("id").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
            };

            if let Some(tid) = task_id
                && let Some(idx) = app
                    .tasks
                    .iter()
                    .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
            {
                app.selected_task_index = idx;
                app.input_mode = InputMode::DeleteTaskConfirm;
            }
        }
        KeyCode::Char('a') => {
            let task_id = {
                let tasks = app.get_kanban_tasks(app.selected_column_index);
                tasks
                    .get(app.selected_kanban_card_index)
                    .and_then(|t| t.get("id").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
            };

            if let Some(tid) = task_id {
                app.archive_task(&tid).await?;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let task_id = {
                let tasks = app.get_kanban_tasks(app.selected_column_index);
                tasks
                    .get(app.selected_kanban_card_index)
                    .and_then(|t| t.get("id").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
            };

            if let Some(tid) = task_id
                && let Some(idx) = app
                    .tasks
                    .iter()
                    .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tid))
            {
                app.selected_task_index = idx;
                app.start_selected_timer().await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_entries_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => app.list_previous(),
        KeyCode::Down | KeyCode::Char('j') => app.list_next(),
        KeyCode::Char('g') => app.selected_entry_index = 0,
        KeyCode::Char('G') => {
            app.selected_entry_index = app.entries.len().saturating_sub(1);
        }
        KeyCode::Char('D') => app.show_entries_for_day().await?,
        KeyCode::Char('W') => app.show_entries_for_week().await?,
        KeyCode::Char('M') => app.show_entries_for_month().await?,
        KeyCode::Char('d') => {
            if !app.entries.is_empty() {
                app.delete_selected_entry().await?;
            }
        }
        KeyCode::Char('e') => {
            if !app.entries.is_empty() {
                app.edit_selected_entry().await?;
            }
        }
        KeyCode::Char('r') => app.refresh_entries().await?,
        KeyCode::Char('f') => {
            app.input_mode = InputMode::FilterEntries;
            app.input_buffer.clear();
            app.status_message = "Enter search term:".to_string();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_reports_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('d') => {
            app.report_period = "day".to_string();
            app.refresh_reports().await?;
        }
        KeyCode::Char('w') => {
            app.report_period = "week".to_string();
            app.refresh_reports().await?;
        }
        KeyCode::Char('m') => {
            app.report_period = "month".to_string();
            app.refresh_reports().await?;
        }
        KeyCode::Char('p') => app.toggle_report_profile().await?,
        KeyCode::Char('r') => app.refresh_reports().await?,
        _ => {}
    }
    Ok(())
}

async fn handle_settings_keys(app: &mut App, code: KeyCode) -> Result<()> {
    use app::SettingsItem;
    let num_settings = SettingsItem::ALL.len();

    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_setting_index = app.selected_setting_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected_setting_index = (app.selected_setting_index + 1).min(num_settings - 1);
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            let selected_item = SettingsItem::ALL[app.selected_setting_index];
            match selected_item {
                SettingsItem::AudioAlerts => app.toggle_audio_alerts(),
                SettingsItem::CowModal => app.toggle_cow_modal(),
                SettingsItem::SyncAutoCommit => app.toggle_git_sync().await?,
                SettingsItem::SyncInitRepo => app.init_git_sync().await?,
                SettingsItem::PomodoroWork => {
                    app.input_mode = InputMode::ConfigPomodoro;
                    app.input_buffer.clear();
                    app.status_message = "Enter work duration in minutes:".to_string();
                }
                SettingsItem::PomodoroShortBreak => {
                    app.input_mode = InputMode::ConfigShortBreak;
                    app.input_buffer.clear();
                    app.status_message = "Enter short break duration in minutes:".to_string();
                }
                SettingsItem::PomodoroLongBreak => {
                    app.input_mode = InputMode::ConfigLongBreak;
                    app.input_buffer.clear();
                    app.status_message = "Enter long break duration in minutes:".to_string();
                }
                SettingsItem::CountdownDefault => {
                    app.input_mode = InputMode::ConfigCountdown;
                    app.input_buffer.clear();
                    app.status_message = "Enter default countdown duration in minutes:".to_string();
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let selected_item = SettingsItem::ALL[app.selected_setting_index];
            match selected_item {
                SettingsItem::PomodoroWork
                | SettingsItem::PomodoroShortBreak
                | SettingsItem::PomodoroLongBreak => {
                    app.adjust_pomodoro_setting(selected_item, -1).await?;
                }
                SettingsItem::CountdownDefault => {
                    if app.countdown_minutes > 1 {
                        app.countdown_minutes -= 1;
                        let seconds = app.countdown_minutes * 60;
                        let _ = app
                            .client
                            .call(
                                "config.update_pomodoro",
                                Some(serde_json::json!({"countdown_default": seconds})),
                            )
                            .await;
                        app.status_message =
                            format!("Countdown default: {} minutes", app.countdown_minutes);
                    }
                }
                _ => {}
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let selected_item = SettingsItem::ALL[app.selected_setting_index];
            match selected_item {
                SettingsItem::PomodoroWork
                | SettingsItem::PomodoroShortBreak
                | SettingsItem::PomodoroLongBreak => {
                    app.adjust_pomodoro_setting(selected_item, 1).await?;
                }
                SettingsItem::CountdownDefault => {
                    app.countdown_minutes += 1;
                    let seconds = app.countdown_minutes * 60;
                    let _ = app
                        .client
                        .call(
                            "config.update_pomodoro",
                            Some(serde_json::json!({"countdown_default": seconds})),
                        )
                        .await;
                    app.status_message =
                        format!("Countdown default: {} minutes", app.countdown_minutes);
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_logs_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_log_index = app.selected_log_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_log_index < app.log_lines.len().saturating_sub(1) {
                app.selected_log_index += 1;
            }
        }
        KeyCode::Char('g') => app.selected_log_index = 0,
        KeyCode::Char('G') => {
            app.selected_log_index = app.log_lines.len().saturating_sub(1);
        }
        KeyCode::Char('r') => {
            app.refresh_logs().await?;
        }
        KeyCode::Char('c') => {
            use mootimer_core::storage::init_data_dir;
            use std::fs;

            let data_dir = init_data_dir()?;
            let log_file_path = data_dir.join("daemon.log");

            if log_file_path.exists() {
                fs::write(&log_file_path, "")?;
                app.log_lines.clear();
                app.status_message = "Logs cleared".to_string();
            }
        }
        _ => {}
    }
    Ok(())
}

async fn startup_sync(client: &MooTimerClient) {
    let sync_future = async {
        let status = match client.call("sync.status", None).await {
            Ok(status) => status,
            Err(_) => return,
        };

        let initialized = status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !initialized {
            return;
        }

        let _ = client.call("sync.sync", None).await;
    };

    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(3), sync_future).await;
}

async fn shutdown_sync(client: &MooTimerClient) {
    let sync_future = async {
        let status = match client.call("sync.status", None).await {
            Ok(status) => status,
            Err(_) => return,
        };

        let initialized = status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !initialized {
            return;
        }

        let _ = client
            .call(
                "sync.commit",
                Some(json!({"message": "Auto-commit on TUI exit"})),
            )
            .await;

        let config = match client.call("config.get", None).await {
            Ok(config) => config,
            Err(_) => return,
        };

        let auto_push = config
            .get("sync")
            .and_then(|s| s.get("auto_push"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !auto_push {
            return;
        }

        let _ = client.call("sync.sync", None).await;
    };

    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), sync_future).await;
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;
    setup_panic_hook();
    info!("MooTimer TUI starting up");

    let args = Args::parse();

    let profile_id = args.profile.unwrap_or_else(|| "default".to_string());

    let client = MooTimerClient::new(&args.socket);

    let profiles = match client.profile_list().await {
        Ok(profiles) => profiles,
        Err(_) => {
            eprintln!("🐮 MooTimer daemon not running. Starting it...");

            let daemon_result = tokio::process::Command::new("mootimerd")
                .arg("--socket")
                .arg(&args.socket)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            match daemon_result {
                Ok(mut child) => {
                    eprintln!("✓ Daemon started (PID: {})", child.id().unwrap_or(0));

                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    match client.profile_list().await {
                        Ok(profiles) => profiles,
                        Err(e) => {
                            eprintln!("✗ Failed to connect to daemon after starting: {}", e);
                            eprintln!("\nTry running manually: mootimerd");
                            let _ = child.kill().await;
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to start daemon: {}", e);
                    eprintln!("\nPlease ensure 'mootimerd' is in your PATH or start it manually:");
                    eprintln!("  cargo run --bin mootimerd");
                    std::process::exit(1);
                }
            }
        }
    };

    let profile_exists = profiles
        .as_array()
        .map(|arr| {
            arr.iter()
                .any(|p| p.get("id").and_then(|id| id.as_str()) == Some(&profile_id))
        })
        .unwrap_or(false);

    if !profile_exists {
        eprintln!("Profile '{}' not found. Creating it...", profile_id);
        match client
            .profile_create(
                &profile_id,
                &profile_id,
                Some(&format!("Default profile - {}", profile_id)),
            )
            .await
        {
            Ok(_) => eprintln!("Profile '{}' created successfully!", profile_id),
            Err(e) => {
                eprintln!("Failed to create profile: {}", e);
                std::process::exit(1);
            }
        }
    }

    startup_sync(&client).await;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut notif_rx = client.subscribe_notifications().await?;

    let mut app = App::new(client, profile_id);

    app.refresh_all().await?;

    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            Some(notification) = notif_rx.recv() => {
                let _ = handle_daemon_notification(&mut app, notification).await;
            }
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                if last_tick.elapsed() >= Duration::from_millis(30) {
                    app.tomato_state.tick();
                    app.cow_state.tick();
                    last_tick = std::time::Instant::now();
                }

                if event::poll(Duration::from_millis(0))? {
                    let event = event::read()?;
                    info!(?event, "Received event");
                    match event {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            handle_key_event(&mut app, key.code, key.modifiers).await?;
                        }
                        Event::Mouse(mouse) => {
                            let (w, h) = size()?;
                            let rect = ratatui::layout::Rect::new(0, 0, w, h);
                            handle_mouse_event(&mut app, mouse, rect).await?;
                        }
                        Event::Resize(width, height) => {
                            info!(width, height, "Terminal resized");
                        },
                        Event::FocusGained | Event::FocusLost => {
                            info!("Terminal focus changed");
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    shutdown_sync(&app.client).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
