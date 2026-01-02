//! MooTimer TUI - Professional Time Tracking Interface

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
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mootimer_client::MooTimerClient;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::json;
use std::io;
use tokio::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "mootimer")]
#[command(about = "MooTimer TUI - Professional work timer", long_about = None)]
struct Args {
    /// Socket path for daemon connection
    #[arg(short, long, default_value = "/tmp/mootimer.sock")]
    socket: String,

    /// Default profile ID
    #[arg(short, long)]
    profile: Option<String>,
}

/// Send an OS notification
fn send_os_notification(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon("clock")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

/// Send an OS notification with urgency
fn send_urgent_notification(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon("alarm-clock")
        .urgency(notify_rust::Urgency::Critical)
        .timeout(notify_rust::Timeout::Milliseconds(10000))
        .show();
}

/// Print bell character for audio alert (respects app.audio_alerts_enabled)
fn audio_alert(app: &App) {
    if app.audio_alerts_enabled {
        print!("\x07"); // Bell character
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

async fn handle_daemon_notification(app: &mut App, notification: mootimer_client::Notification) -> Result<()> {
    match notification.method.as_str() {
        "timer.event" => {
            // Parse timer event from params
            if let Some(event_type) = notification.params.get("event_type") {
                if let Some(event_type_obj) = event_type.as_object() {
                    if let Some(type_str) = event_type_obj.get("type").and_then(|v| v.as_str()) {
                        match type_str {
                            "tick" => {
                                // Timer tick - check for 5-minute warning on countdown timers
                                if let Some(remaining) = event_type_obj.get("remaining_seconds").and_then(|v| v.as_u64()) {
                                // If countdown timer has 5 minutes remaining and we haven't shown warning yet
                                if remaining <= 300 && remaining > 295 && !app.five_min_warning_shown {
                                    app.five_min_warning_shown = true;
                                    app.status_message = "⚠️  5 minutes remaining!".to_string();
                                    audio_alert(app);
                                    send_os_notification(
                                        "⏰ 5 Minutes Left",
                                        "Your countdown timer is almost complete!"
                                    );
                                }
                                }
                                
                                // Timer tick - refresh timer display
                                app.refresh_timer().await?;
                            }
                            "started" => {
                                app.status_message = "Timer started".to_string();
                                app.five_min_warning_shown = false; // Reset warning for new timer
                                app.refresh_timer().await?;
                            }
                            "stopped" => {
                                app.status_message = "Timer stopped, entry saved!".to_string();
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
                                // COUNTDOWN COMPLETION - Show notification!
                                app.status_message = "🔔 COUNTDOWN COMPLETED! 🔔".to_string();
                                
                                // Audio alert
                                audio_alert(app);
                                
                                // OS notification
                                send_urgent_notification(
                                    "⏰ Countdown Complete!",
                                    "Your countdown timer has finished."
                                );
                                
                                app.refresh_timer().await?;
                                app.refresh_stats().await?;
                                app.refresh_entries().await?;
                            }
                            "phase_completed" => {
                                // Pomodoro phase completed
                                if let Some(phase_obj) = event_type_obj.get("phase") {
                                    let phase = phase_obj.as_str().unwrap_or("unknown");
                                    app.status_message = format!("🍅 {} phase completed!", phase);
                                    
                                    // Audio alert
                                    audio_alert(app);
                                    
                                    // OS notification
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
                                // Pomodoro phase changed
                                if let Some(phase_obj) = event_type_obj.get("new_phase") {
                                    let phase = phase_obj.as_str().unwrap_or("unknown");
                                    app.status_message = format!("🍅 Starting {} phase", phase);
                                    
                                    // Show notification for break start (less intrusive for work start)
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
            }
        }
        _ => {}
    }
    
    Ok(())
}

async fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    // Profile Manager modal handling - check FIRST before general input mode handling
    if app.input_mode == InputMode::ProfileManager {
        match code {
            KeyCode::Char('P') if modifiers.contains(KeyModifiers::SHIFT) => {
                app.input_mode = InputMode::Normal;
                return Ok(());
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                return Ok(());
            }
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
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Char('n') => {
                app.input_mode = InputMode::NewProfile;
                app.input_buffer.clear();
                app.status_message = "Enter profile name:".to_string();
            }
            KeyCode::Char('d') => {
                app.delete_selected_profile().await?;
            }
            KeyCode::Char('r') => {
                app.input_mode = InputMode::RenameProfile;
                app.input_buffer.clear();
                app.status_message = "Enter new profile name:".to_string();
            }
            _ => {}
        }
        return Ok(());
    }

    // Handle text input modes (NewTask, EditTask, NewProfile, RenameProfile, etc.)
    if app.input_mode != InputMode::Normal && app.input_mode != InputMode::ProfileManager {
        match code {
            KeyCode::Enter => {
                app.submit_input().await?;
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input_buffer.clear();
            }
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    // Allow Ctrl+C even in input mode
                    if c == 'c' {
                        app.should_quit = true;
                    }
                } else {
                    app.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            _ => {}
        }
        return Ok(());
    }

    // Normal mode key handling
    match code {
        // Global shortcuts
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.show_help {
                app.toggle_help();
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_view == AppView::Dashboard {
                app.toggle_dashboard_pane();
            }
        }

        // 't' key for timer type cycling (Dashboard only)
        KeyCode::Char('t') => {
            if app.current_view == AppView::Dashboard && app.focused_pane == app::DashboardPane::TimerConfig {
                app.cycle_timer_type();
            }
        }
        KeyCode::Char('T') if modifiers.contains(KeyModifiers::SHIFT) => {
            if app.current_view == AppView::Dashboard && app.focused_pane == app::DashboardPane::TimerConfig {
                app.cycle_timer_type_reverse();
            }
        }
        KeyCode::Char('1') => {
            app.current_view = AppView::Dashboard;
            app.refresh_tasks().await?;
        }
        KeyCode::Char('2') => {
            app.current_view = AppView::Entries;
            app.refresh_entries().await?;
        }
        KeyCode::Char('3') => {
            app.current_view = AppView::Reports;
            app.refresh_tasks().await?; // Need tasks for task breakdown
            app.refresh_reports().await?;
        }
        KeyCode::Char('4') => app.current_view = AppView::Settings,
        KeyCode::Char('5') => {
            app.current_view = AppView::Logs;
            app.refresh_logs().await?;
        }
        KeyCode::Char('P') if modifiers.contains(KeyModifiers::SHIFT) => {
            // Shift+P opens profile manager modal
            app.input_mode = InputMode::ProfileManager;
            app.refresh_profiles().await?;
        }

        // Navigation (Left/Right for buttons, PageUp/PageDown for lists)
        // Note: Up/Down/j/k are now handled per-view for better context awareness
        KeyCode::Left | KeyCode::Char('h') => app.button_previous(),
        KeyCode::Right | KeyCode::Char('l') => app.button_next(),
        KeyCode::PageUp => app.list_page_up(),
        KeyCode::PageDown => app.list_page_down(),

        // Enter key activates the currently selected button
        KeyCode::Enter => {
            activate_selected_button(app).await?;
        }

        // View-specific actions
        _ => match app.current_view {
            AppView::Dashboard => handle_dashboard_keys(app, code).await?,
            AppView::Entries => handle_entries_keys(app, code).await?,
            AppView::Reports => handle_reports_keys(app, code).await?,
            AppView::Settings => handle_settings_keys(app, code).await?,
            AppView::Logs => handle_logs_keys(app, code).await?,

        },
    }

    Ok(())
}

/// Activates the currently selected button by simulating the button's shortcut key
async fn activate_selected_button(app: &mut App) -> Result<()> {
    use crate::app::{AppView, DashboardPane};
    
    let button_index = app.selected_button_index;
    
    match app.current_view {
        AppView::Dashboard => {
            match app.focused_pane {
                DashboardPane::TimerConfig => {
                    let timer_state = app
                        .timer_info
                        .as_ref()
                        .and_then(|t| t.get("state"))
                        .and_then(|s| s.as_str());
                    
                    match timer_state {
                        Some("running") | Some("paused") => {
                            // Buttons: Pause/Resume (0), Stop (1)
                            match button_index {
                                0 => app.toggle_pause().await?,
                                1 => app.stop_timer().await?,
                                _ => {}
                            }
                        }
                        _ => {
                            // Buttons: Start Timer (0), Type (1)
                            match button_index {
                                0 => app.start_selected_timer().await?,
                                1 => app.cycle_timer_type(),
                                _ => {}
                            }
                        }
                    }
                }
                DashboardPane::TasksList => {
                    // Buttons: New (0), Edit (1), Delete (2), Start Timer (3)
                    match button_index {
                        0 => {
                            app.input_mode = InputMode::NewTask;
                            app.input_buffer.clear();
                            app.status_message = "Enter task title:".to_string();
                        }
                        1 => app.edit_selected_task().await?,
                        2 => app.delete_selected_task().await?,
                        3 => app.start_selected_timer().await?,
                        _ => {}
                    }
                }
            }
        }
        AppView::Entries => {
            // Buttons: Today (0), This Week (1), This Month (2), Refresh (3)
            match button_index {
                0 => app.show_entries_for_day().await?,
                1 => app.show_entries_for_week().await?,
                2 => app.show_entries_for_month().await?,
                3 => app.refresh_entries().await?,
                _ => {}
            }
        }
        AppView::Reports => {
            // Buttons: Daily (0), Weekly (1), Monthly (2), Toggle Profile (3), Refresh (4)
            match button_index {
                0 => {
                    app.report_period = "day".to_string();
                    app.refresh_reports().await?;
                }
                1 => {
                    app.report_period = "week".to_string();
                    app.refresh_reports().await?;
                }
                2 => {
                    app.report_period = "month".to_string();
                    app.refresh_reports().await?;
                }
                3 => app.toggle_report_profile().await?,
                4 => app.refresh_reports().await?,
                _ => {}
            }
        }
        AppView::Settings => {
            // No buttons in Settings
        }
        AppView::Logs => {
            // Buttons: Refresh (0), Clear (1)
            match button_index {
                0 => app.refresh_logs().await?,
                1 => {
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
        }
    }
    
    Ok(())
}

async fn handle_dashboard_keys(app: &mut App, code: KeyCode) -> Result<()> {
    use crate::app::DashboardPane;

    // Context-aware keybinds based on which pane is focused
    match app.focused_pane {
        DashboardPane::TimerConfig => {
            // Timer pane is focused - handle timer-specific controls
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    // Adjust duration up (only if no timer running)
                    if app.timer_info.is_none() {
                        app.adjust_timer_duration_up();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // Adjust duration down (only if no timer running)
                    if app.timer_info.is_none() {
                        app.adjust_timer_duration_down();
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Start/pause/resume timer
                    if let Some(timer) = &app.timer_info {
                        if let Some(state) = timer.get("state").and_then(|v| v.as_str()) {
                            if state == "running" || state == "paused" {
                                app.toggle_pause().await?;
                                return Ok(());
                            }
                        }
                    }
                    app.start_selected_timer().await?;
                }
                KeyCode::Char('x') => {
                    // Stop timer (works regardless of focus)
                    app.stop_timer().await?;
                }
                KeyCode::Char('r') => {
                    // Refresh (works regardless of focus)
                    app.status_message = "Refreshing...".to_string();
                    app.refresh_all().await?;
                    app.status_message = "Refreshed!".to_string();
                }
                // Note: Tab is handled at the global level for timer type cycling
                _ => {}
            }
        }
        DashboardPane::TasksList => {
            // Tasks pane is focused - handle task-specific controls
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.list_previous();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.list_next();
                }
                KeyCode::Char('g') => {
                    // Jump to top (vim style)
                    app.selected_task_index = 0;
                }
                KeyCode::Char('G') => {
                    // Jump to bottom (vim style)
                    app.selected_task_index = app.tasks.len().saturating_sub(1);
                }
                KeyCode::Char('n') => {
                    // New task
                    app.input_mode = InputMode::NewTask;
                    app.input_buffer.clear();
                    app.status_message = "Enter task title:".to_string();
                }
                KeyCode::Char('d') => {
                    // Delete selected task
                    app.delete_selected_task().await?;
                }
                KeyCode::Char('e') => {
                    // Edit selected task
                    app.edit_selected_task().await?;
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // When in task list, Enter/Space starts timer with selected task
                    if let Some(timer) = &app.timer_info {
                        if let Some(state) = timer.get("state").and_then(|v| v.as_str()) {
                            if state == "running" || state == "paused" {
                                app.toggle_pause().await?;
                                return Ok(());
                            }
                        }
                    }
                    app.start_selected_timer().await?;
                }
                KeyCode::Char('x') => {
                    // Stop timer (works regardless of focus)
                    app.stop_timer().await?;
                }
                KeyCode::Char('r') => {
                    // Refresh (works regardless of focus)
                    app.status_message = "Refreshing...".to_string();
                    app.refresh_all().await?;
                    app.status_message = "Refreshed!".to_string();
                }
                _ => {}
            }
        }
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
        KeyCode::Char('d') => app.show_entries_for_day().await?,
        KeyCode::Char('w') => app.show_entries_for_week().await?,
        KeyCode::Char('m') => app.show_entries_for_month().await?,
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
    match code {
        KeyCode::Char('p') => {
            app.input_mode = InputMode::ConfigPomodoro;
            app.input_buffer.clear();
            app.status_message = "Enter work duration in minutes:".to_string();
        }
        KeyCode::Char('b') => {
            app.input_mode = InputMode::ConfigShortBreak;
            app.input_buffer.clear();
            app.status_message = "Enter short break duration in minutes:".to_string();
        }
        KeyCode::Char('l') => {
            app.input_mode = InputMode::ConfigLongBreak;
            app.input_buffer.clear();
            app.status_message = "Enter long break duration in minutes:".to_string();
        }
        KeyCode::Char('g') => {
            app.toggle_git_sync().await?;
        }
        KeyCode::Char('i') => {
            app.init_git_sync().await?;
        }
        KeyCode::Char('a') => {
            app.toggle_audio_alerts();
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

/// Sync on startup - pull latest changes from remote if sync is enabled
async fn startup_sync(client: &MooTimerClient) {
    // Wrap entire sync in a timeout to prevent blocking startup
    let sync_future = async {
        // Check if sync is initialized
        let status = match client.call("sync.status", None).await {
            Ok(status) => status,
            Err(_) => return, // Silently skip if sync.status fails
        };

        let initialized = status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !initialized {
            // Git not initialized, skip sync (silent)
            return;
        }

        // Try to sync (pull latest changes) - silently
        let _ = client.call("sync.sync", None).await;
    };

    // Timeout after 3 seconds to avoid blocking startup
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(3), sync_future).await;
}

/// Sync on shutdown - commit and push changes if auto-sync is enabled
async fn shutdown_sync(client: &MooTimerClient) {
    // Wrap entire sync in a timeout to prevent blocking exit
    let sync_future = async {
        // Check if sync is initialized
        let status = match client.call("sync.status", None).await {
            Ok(status) => status,
            Err(_) => return, // Silently skip if sync.status fails
        };

        let initialized = status
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !initialized {
            // Git not initialized, skip sync
            return;
        }

        // Auto-commit on exit (silently)
        let _ = client
            .call(
                "sync.commit",
                Some(json!({"message": "Auto-commit on TUI exit"})),
            )
            .await;

        // Check if auto_push is enabled
        let config = match client.call("config.get", None).await {
            Ok(config) => config,
            Err(_) => return, // Can't get config, skip push
        };

        let auto_push = config
            .get("sync")
            .and_then(|s| s.get("auto_push"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !auto_push {
            // Auto-push disabled, we're done
            return;
        }

        // Auto-push to remote (silently)
        let _ = client.call("sync.sync", None).await;
    };

    // Timeout after 5 seconds to avoid blocking exit
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), sync_future).await;
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Get default profile or use "default"
    let profile_id = args.profile.unwrap_or_else(|| "default".to_string());

    // Create client
    let client = MooTimerClient::new(&args.socket);

    // Track daemon child process if WE started it (so we can clean up on exit)
    let mut daemon_child: Option<tokio::process::Child> = None;

    // Try to connect and get profile list
    let profiles = match client.profile_list().await {
        Ok(profiles) => profiles,
        Err(_) => {
            // Daemon not running, try to start it
            eprintln!("🐮 MooTimer daemon not running. Starting it...");

            // Try to find mootimerd in PATH
            let daemon_result = tokio::process::Command::new("mootimerd")
                .arg("--socket")
                .arg(&args.socket)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            match daemon_result {
                Ok(mut child) => {
                    eprintln!("✓ Daemon started (PID: {})", child.id().unwrap_or(0));

                    // Give daemon time to start
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    // Try connecting again
                    match client.profile_list().await {
                        Ok(profiles) => {
                            // Success! Store child process for later cleanup
                            daemon_child = Some(child);
                            profiles
                        }
                        Err(e) => {
                            eprintln!("✗ Failed to connect to daemon after starting: {}", e);
                            eprintln!("\nTry running manually: mootimerd");
                            let _ = child.kill().await; // Kill the daemon we just started
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

    // Check if the profile exists, if not create it
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

    // Sync on startup - pull latest changes if sync is enabled
    startup_sync(&client).await;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Subscribe to daemon notifications BEFORE creating app (moves client)
    let mut notif_rx = client.subscribe_notifications().await?;

    // Create app state (moves client)
    let mut app = App::new(client, profile_id);

    // Initial data load
    app.refresh_all().await?;

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        // Event-driven loop - wait for daemon events or keyboard input
        tokio::select! {
            // Daemon notifications (timer events, etc)
            Some(notification) = notif_rx.recv() => {
                let _ = handle_daemon_notification(&mut app, notification).await;
            }
            // Keyboard events
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                // Check for keyboard events (60 FPS max)
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            handle_key_event(&mut app, key.code, key.modifiers).await?;
                        }
                    }
                }
            }
        }
    }

    // Sync on shutdown - commit and push changes if enabled
    shutdown_sync(&app.client).await;

    // Shutdown daemon if we started it
    if let Some(mut child) = daemon_child {
        eprintln!("🐮 Shutting down daemon...");
        let _ = child.kill().await;
        
        // Wait for it to exit gracefully (with timeout)
        let shutdown_timeout = tokio::time::Duration::from_secs(2);
        match tokio::time::timeout(shutdown_timeout, child.wait()).await {
            Ok(Ok(status)) => eprintln!("✓ Daemon stopped ({})", status),
            Ok(Err(e)) => eprintln!("⚠ Daemon exit error: {}", e),
            Err(_) => eprintln!("⚠ Daemon shutdown timeout"),
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
