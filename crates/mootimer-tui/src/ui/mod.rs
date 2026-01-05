//! UI rendering module

mod buttons;
mod confirmation;
pub mod cow;
mod dashboard;
mod entries;
mod input;
mod kanban;
mod logs;
mod reports;
mod settings;
pub mod tomato;

use crate::app::{App, AppView, InputMode};
use buttons::{Button, render_button_row};
use confirmation::draw_confirmation_modal;
use dashboard::draw_dashboard;
use entries::draw_entries;
use input::draw_input_modal;
use kanban::draw_kanban;
use logs::draw_logs;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use reports::draw_reports;
use settings::draw_settings;

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
        AppView::Kanban => draw_kanban(f, app, chunks[1]),
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

    // Confirmation modals
    if app.input_mode == InputMode::DeleteTaskConfirm
        || app.input_mode == InputMode::DeleteProfileConfirm
    {
        draw_confirmation_modal(f, app);
    }

    // Input modals (NewTask, EditTask, etc.)
    match app.input_mode {
        InputMode::NewTask
        | InputMode::EditTask
        | InputMode::SearchTasks
        | InputMode::FilterEntries
        | InputMode::ConfigPomodoro
        | InputMode::ConfigShortBreak
        | InputMode::ConfigLongBreak
        | InputMode::ConfigCountdown
        | InputMode::NewProfile
        | InputMode::RenameProfile
        | InputMode::EditEntryDuration => {
            draw_input_modal(f, app);
        }
        _ => {}
    }

    // Cow modal (drawn last, on top of everything!)
    if app.show_cow_modal {
        draw_cow_modal(f);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let tabs = [
        ("1", "📊", "Dashboard", AppView::Dashboard),
        ("2", "📋", "Kanban", AppView::Kanban),
        ("3", "📝", "Entries", AppView::Entries),
        ("4", "📈", "Reports", AppView::Reports),
        ("5", "⚙️", "Settings", AppView::Settings),
        ("6", "📋", "Logs", AppView::Logs),
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
                DashboardPane::ProfileList => {
                    // Profile pane focused - show profile controls
                    vec![
                        Button::new("Switch", "Enter", app.selected_button_index == 0),
                        Button::new("New", "n", app.selected_button_index == 1),
                        Button::new("Delete", "d", app.selected_button_index == 2),
                        Button::new("Rename", "r", app.selected_button_index == 3),
                    ]
                }
            }
        }
        AppView::Kanban => vec![
            Button::new("New Task", "n", app.selected_button_index == 0),
            Button::new("Edit Task", "e", app.selected_button_index == 1),
            Button::new("Delete Task", "d", app.selected_button_index == 2),
            Button::new("Start Timer", "Space", app.selected_button_index == 3),
        ],
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
            "    [1-6]        Jump to view (1=Dash 2=Kanban 3=Entries 4=Reports 5=Settings 6=Logs)",
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
        Line::from("    [a]          Archive/Un-archive selected task"),
        Line::from("    [Shift+A]    Toggle view of archived tasks"),
        Line::from("    [v]          Toggle task descriptions"),
        Line::from("    [e]          Edit selected task"),
        Line::from(""),
        Line::from(Span::styled(
            "  KANBAN",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [h/l]        Navigate columns"),
        Line::from("    [j/k]        Navigate tasks in a column"),
        Line::from("    [Shift+h/l]  Move task to adjacent column"),
        Line::from("    [v]          Toggle task descriptions"),
        Line::from("    [a]          Archive selected task"),
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
        Line::from("    [↑↓]/[j/k]   Navigate settings"),
        Line::from("    [Space/Enter]Toggle or Edit selected setting"),
        Line::from("    [h/l]        Decrement/Increment value (for durations)"),
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

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.input_mode {
        InputMode::Normal => {
            if !app.status_message.is_empty() {
                app.status_message.clone()
            } else {
                match app.current_view {
                    AppView::Dashboard => {
                        "[Ctrl+w]Focus • [j/k]Nav • [/]Search • [A]rchives".to_string()
                    }
                    AppView::Kanban => {
                        "[h/l]Cols • [j/k]Cards • [Shift+h/l]Move • [a]Archive".to_string()
                    }
                    _ => "[1-6]Views • [↑↓/j/k]Navigate • [←→/h/l]Buttons • [q]Quit".to_string(),
                }
            }
        }
        InputMode::DeleteTaskConfirm | InputMode::DeleteProfileConfirm => {
            "Confirm Action: [Y]es / [N]o".to_string()
        }
        _ => {
            // Text input modes (NewTask, etc.) - Modal is shown
            "[Enter] Submit  [Esc] Cancel".to_string()
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(status, area);
}

fn draw_cow_modal(f: &mut Frame) {
    let cow_art = vec![
        "",
        "  _______________________________________",
        " /                                       \\",
        "|   🎉 TIME'S UP! BREAK TIME! 🎉         |",
        "|                                         |",
        "|   Your countdown has completed!         |",
        "|   Go stretch, hydrate, and relax! 🧘   |",
        " \\_______________________________________/",
        "        \\   ^__^",
        "         \\  (oo)\\_______",
        "            (__)\\       )\\/\\",
        "                ||----w |",
        "                ||     ||",
        "",
        "         🐮 Moo-tastic work! 🐮",
        "",
        "     Press any key to continue...",
        "",
    ];

    let area = f.area();
    let modal_height = cow_art.len() as u16 + 4;
    let modal_width = 50;

    // Center the modal
    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    // Create the cow text
    let cow_text: Vec<Line> = cow_art
        .iter()
        .map(|line| Line::from(Span::raw(*line)))
        .collect();

    let cow_paragraph = Paragraph::new(cow_text)
        .style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
                .border_type(ratatui::widgets::BorderType::Double),
        );

    // Clear the area first (semi-transparent background effect)
    f.render_widget(Clear, modal_area);
    f.render_widget(cow_paragraph, modal_area);
}
