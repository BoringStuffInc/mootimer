mod buttons;
mod confirmation;
pub mod cow;
mod dashboard;
mod entries;
pub mod helpers;
mod input;
mod kanban;
mod logs;
mod reports;
mod settings;
mod timers;
pub mod tomato;

use crate::app::{App, AppView, InputMode};
use confirmation::{draw_break_finished_modal, draw_confirmation_modal};
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
use timers::draw_timers;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);

    match app.current_view {
        AppView::Dashboard => draw_dashboard(f, app, chunks[1]),
        AppView::Timers => draw_timers(f, app, chunks[1]),
        AppView::Kanban => draw_kanban(f, app, chunks[1]),
        AppView::Entries => draw_entries(f, app, chunks[1]),
        AppView::Reports => draw_reports(f, app, chunks[1]),
        AppView::Settings => draw_settings(f, app, chunks[1]),
        AppView::Logs => draw_logs(f, app, chunks[1]),
    }

    draw_status_bar(f, app, chunks[2]);

    if app.show_help {
        draw_help_modal(f, app);
    }

    if app.input_mode == InputMode::DeleteTaskConfirm
        || app.input_mode == InputMode::DeleteProfileConfirm
        || app.input_mode == InputMode::ConfirmQuit
    {
        draw_confirmation_modal(f, app);
    }

    if app.input_mode == InputMode::PomodoroBreakFinished {
        draw_break_finished_modal(f);
    }

    match app.input_mode {
        InputMode::NewTask
        | InputMode::QuickAddTask
        | InputMode::EditTask
        | InputMode::SearchTasks
        | InputMode::FilterEntries
        | InputMode::ConfigPomodoro
        | InputMode::ConfigShortBreak
        | InputMode::ConfigLongBreak
        | InputMode::ConfigCountdown
        | InputMode::NewProfile
        | InputMode::RenameProfile
        | InputMode::EditEntryDuration
        | InputMode::NewEntryStart
        | InputMode::NewEntryEnd
        | InputMode::NewEntryDescription => {
            draw_input_modal(f, app);
        }
        InputMode::MoveTask => {
            draw_move_task_modal(f, app);
        }
        InputMode::NewEntryTask => {
            draw_task_select_modal(f, app);
        }
        _ => {}
    }

    if app.show_cow_modal {
        draw_cow_modal(f);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let tabs = [
        ("1", "📊", "Dashboard", AppView::Dashboard),
        ("2", "⏱️", "Timers", AppView::Timers),
        ("3", "📋", "Kanban", AppView::Kanban),
        ("4", "📝", "Entries", AppView::Entries),
        ("5", "📈", "Reports", AppView::Reports),
        ("6", "⚙️", "Settings", AppView::Settings),
        ("7", "📋", "Logs", AppView::Logs),
    ];

    let mut spans = vec![
        Span::styled(
            "🐮 MooTimer ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("[{}] │ ", app.get_profile_name())),
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

fn draw_help_modal(f: &mut Frame, _app: &App) {
    use ratatui::{
        text::{Line, Span},
        widgets::Clear,
    };

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

    f.render_widget(Clear, modal_area);

    let help_text = vec![
        Line::from(Span::styled(
            "  🐮 MooTimer - Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  GLOBAL NAVIGATION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [1]          Dashboard (📊)"),
        Line::from("    [2]          Active Timers (⏱️)"),
        Line::from("    [3]          Kanban Board (📋)"),
        Line::from("    [4]          Entries Log (📝)"),
        Line::from("    [5]          Reports (📈)"),
        Line::from("    [6]          Settings (⚙️)"),
        Line::from("    [7]          System Logs (📋)"),
        Line::from("    [m]          Moo! (🐮)"),
        Line::from("    [?]          Toggle this Help"),
        Line::from("    [q] / [Esc]  Quit MooTimer"),
        Line::from(""),
        Line::from(Span::styled(
            "  DASHBOARD - TIMER (Focus with Ctrl+w)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [Space]      Start / Pause / Resume Timer"),
        Line::from("    [x]          Stop and Save Timer Entry"),
        Line::from("    [t]          Cycle Timer Type (Manual → Pomodoro → Countdown)"),
        Line::from("    [↑↓] / [j/k] Adjust Duration (Pomodoro/Countdown only, when idle)"),
        Line::from("    [r]          Refresh Timer Status"),
        Line::from(""),
        Line::from(Span::styled(
            "  DASHBOARD - TASKS (Focus with Ctrl+w)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [n]          Create New Task (Full: Title + Description)"),
        Line::from("    [N]          Quick Add Task (Title only)"),
        Line::from("    [e]          Edit Selected Task Title"),
        Line::from("    [d]          Delete Selected Task"),
        Line::from("    [a]          Archive / Restore Selected Task"),
        Line::from("    [Shift+A]    Toggle View: Active vs. Archived Tasks"),
        Line::from("    [v]          Toggle Visibility of Task Descriptions"),
        Line::from("    [/]          Search Tasks"),
        Line::from("    [m]          Move Task to Another Profile"),
        Line::from("    [↑↓] / [j/k] Navigate Tasks"),
        Line::from("    [g]g / [G]   Jump to Top / Bottom"),
        Line::from(""),
        Line::from(Span::styled(
            "  KANBAN BOARD",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [h/l]        Switch Column (To Do / In Progress / Done)"),
        Line::from("    [j/k]        Navigate Cards in Column"),
        Line::from("    [Shift+h/l]  Move Card to Adjacent Column"),
        Line::from("    [n]          Create New Task (Full)"),
        Line::from("    [N]          Quick Add Task (Title only)"),
        Line::from("    [a]          Archive / Restore Selected Card"),
        Line::from("    [Shift+A]    Toggle View: Active vs. Archived Cards"),
        Line::from("    [v]          Toggle Card Descriptions"),
        Line::from(""),
        Line::from(Span::styled(
            "  ENTRIES LOG",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [d] / [w] / [m]  Filter by Today / Week / Month"),
        Line::from("    [f]              Custom Text Filter"),
        Line::from("    [e]              Edit Selected Entry Duration"),
        Line::from("    [d] (Delete)     Delete Selected Entry"),
        Line::from("    [r]              Refresh Entries"),
        Line::from(""),
        Line::from(Span::styled(
            "  REPORTS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [d] / [w] / [m]  Switch Report Period (Daily / Weekly / Monthly)"),
        Line::from("    [p]              Toggle All Profiles vs. Current Profile"),
        Line::from("    [r]              Refresh Report Data"),
        Line::from(""),
        Line::from(Span::styled(
            "  PROFILE MANAGER (Shift+P)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("    [Shift+P]    Open / Close Profile Manager"),
        Line::from("    [Enter]      Switch to Selected Profile"),
        Line::from("    [n]          Create New Profile"),
        Line::from("    [d]          Delete Selected Profile"),
        Line::from("    [r]          Rename Selected Profile"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close this help",
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
    let profile_name = app.get_profile_name();

    let active_task = if let Some(timer) = &app.timer_info {
        timer
            .get("task_id")
            .and_then(|id| {
                app.tasks
                    .iter()
                    .find(|t| t.get("id") == Some(id))
                    .and_then(|t| t.get("title"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("No active task")
    } else {
        "No active timer"
    };

    let status_content = if !app.status_message.is_empty() {
        let bg_color = if app.status_message.contains("MOO") {
            Color::Magenta
        } else {
            Color::Yellow
        };
        Span::styled(
            format!(" {} ", app.status_message),
            Style::default().fg(Color::Black).bg(bg_color),
        )
    } else {
        match app.input_mode {
            InputMode::Normal => {
                let hints = match app.current_view {
                    AppView::Dashboard => "[1-6]Views • [?]Help • [q]Quit",
                    AppView::Kanban => "[1-6]Views • [h/l]Col • [j/k]Card • [m]Move • [a]Arch",
                    _ => "[1-6]Views • [↑↓/j/k]Nav • [q]Quit",
                };
                Span::raw(hints)
            }
            InputMode::DeleteTaskConfirm | InputMode::DeleteProfileConfirm => Span::styled(
                " Confirm: [Y]es / [N]o ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            _ => Span::raw("[Enter] Submit  [Esc] Cancel"),
        }
    };

    let left_info = Span::styled(
        format!(" 👤 {} ", profile_name),
        Style::default().fg(Color::Cyan),
    );
    let center_info = Span::styled(
        format!(" 🎯 {} ", active_task),
        Style::default().fg(Color::Gray),
    );

    let sync_info = if let Some(sync) = &app.sync_status {
        let initialized = sync
            .get("initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if initialized {
            let ahead = sync.get("ahead").and_then(|v| v.as_u64()).unwrap_or(0);
            let behind = sync.get("behind").and_then(|v| v.as_u64()).unwrap_or(0);
            let changes = sync
                .get("has_changes")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut sync_spans = vec![Span::raw(" │ ")];

            if ahead > 0 || behind > 0 || changes {
                let mut details = Vec::new();
                if ahead > 0 {
                    details.push(format!("↑{}", ahead));
                }
                if behind > 0 {
                    details.push(format!("↓{}", behind));
                }
                if changes {
                    details.push("*".to_string());
                }

                sync_spans.push(Span::styled(
                    format!(" ☁ {} ", details.join(" ")),
                    Style::default().fg(Color::Yellow),
                ));
            } else {
                sync_spans.push(Span::styled(" ☁ ok ", Style::default().fg(Color::Green)));
            }
            sync_spans
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let mut status_spans = vec![
        left_info,
        Span::raw(" │ "),
        center_info,
        Span::raw(" │ "),
        status_content,
    ];

    if !sync_info.is_empty() {
        status_spans.extend(sync_info);
    }

    let status_line = Line::from(status_spans);

    let status = Paragraph::new(status_line)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    f.render_widget(status, area);
}

fn draw_task_select_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let modal_width = 60.min(area.width.saturating_sub(4));
    let tasks = app.get_tasks_for_entry_selection();
    let modal_height = (tasks.len() as u16 + 8)
        .min(area.height.saturating_sub(4))
        .max(10);

    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    let mut items: Vec<ratatui::widgets::ListItem> = vec![{
        let is_selected = app.new_entry_task_index == 0;
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "→ " } else { "  " };
        ratatui::widgets::ListItem::new(format!("{}(No task)", prefix)).style(style)
    }];

    for (i, task) in tasks.iter().enumerate() {
        let title = task
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let is_archived = status == "archived";
        let is_selected = app.new_entry_task_index == i + 1;

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_archived {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if is_selected { "→ " } else { "  " };
        let archived_marker = if is_archived { " [archived]" } else { "" };
        let display = format!("{}{}{}", prefix, title, archived_marker);
        let truncated = if display.len() > modal_width as usize - 4 {
            format!("{}...", &display[..modal_width as usize - 7])
        } else {
            display
        };
        items.push(ratatui::widgets::ListItem::new(truncated).style(style));
    }

    let archived_hint = if app.new_entry_show_archived {
        "[a] Hide archived"
    } else {
        "[a] Show archived"
    };
    let hint = format!(" [j/k]Select [Enter]Choose [Esc]Skip | {} ", archived_hint);

    let list = ratatui::widgets::List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Task (optional) ")
            .title_bottom(Line::from(hint).right_aligned())
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    f.render_widget(list, modal_area);
}

fn draw_move_task_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let modal_width = 50.min(area.width.saturating_sub(4));
    let profiles = app.get_move_task_profiles();
    let modal_height = (profiles.len() as u16 + 6).min(area.height.saturating_sub(4));

    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    let task_title = app
        .get_filtered_tasks()
        .get(app.selected_task_index)
        .and_then(|t| t.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown task");

    let title = format!(" Move: {} ", task_title);
    let truncated_title = if title.len() > modal_width as usize - 2 {
        format!("{}...", &title[..modal_width as usize - 5])
    } else {
        title
    };

    let items: Vec<ratatui::widgets::ListItem> = profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let name = profile
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let is_selected = i == app.move_task_target_index;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_selected { "→ " } else { "  " };
            ratatui::widgets::ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    let list = ratatui::widgets::List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(truncated_title)
            .title_bottom(Line::from(" [j/k]Select [Enter]Move [Esc]Cancel ").right_aligned())
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    f.render_widget(list, modal_area);
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

    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

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

    f.render_widget(Clear, modal_area);
    f.render_widget(cow_paragraph, modal_area);
}
pub mod big_text;
