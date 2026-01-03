use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::SettingsItem;

    let items: Vec<ListItem> = SettingsItem::ALL
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let is_selected = i == app.selected_setting_index;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let line = match item {
                SettingsItem::PomodoroWork => {
                    let val = app
                        .config
                        .as_ref()
                        .and_then(|c| c.get("pomodoro").and_then(|p| p.get("work_duration")))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        / 60;
                    Line::from(vec![
                        Span::styled(
                            "Work Duration   ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{} minutes", val)),
                        if is_selected {
                            Span::raw("  <[h/l] to change>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::PomodoroShortBreak => {
                    let val = app
                        .config
                        .as_ref()
                        .and_then(|c| c.get("pomodoro").and_then(|p| p.get("short_break")))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        / 60;
                    Line::from(vec![
                        Span::styled(
                            "Short Break     ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{} minutes", val)),
                        if is_selected {
                            Span::raw("  <[h/l] to change>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::PomodoroLongBreak => {
                    let val = app
                        .config
                        .as_ref()
                        .and_then(|c| c.get("pomodoro").and_then(|p| p.get("long_break")))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        / 60;
                    Line::from(vec![
                        Span::styled(
                            "Long Break      ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{} minutes", val)),
                        if is_selected {
                            Span::raw("  <[h/l] to change>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::AudioAlerts => {
                    let val = if app.audio_alerts_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    };
                    Line::from(vec![
                        Span::styled(
                            "Audio Alerts    ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(val),
                        if is_selected {
                            Span::raw("  <[Space] to toggle>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::CountdownDefault => {
                    let val = app.countdown_minutes;
                    Line::from(vec![
                        Span::styled(
                            "Countdown Timer ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{} minutes", val)),
                        if is_selected {
                            Span::raw("  <[h/l] to change>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::CowModal => {
                    let val = if app.cow_modal_enabled {
                        "Enabled 🐮"
                    } else {
                        "Disabled"
                    };
                    Line::from(vec![
                        Span::styled(
                            "Cow Modal       ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(val),
                        if is_selected {
                            Span::raw("  <[Space] to toggle>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::SyncAutoCommit => {
                    let val = app
                        .config
                        .as_ref()
                        .and_then(|c| c.get("sync").and_then(|s| s.get("auto_commit")))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Line::from(vec![
                        Span::styled(
                            "Auto-Commit     ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(if val { "Enabled" } else { "Disabled" }),
                        if is_selected {
                            Span::raw("  <[Space] to toggle>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
                SettingsItem::SyncInitRepo => {
                    let val = app
                        .sync_status
                        .as_ref()
                        .and_then(|s| s.get("initialized"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Line::from(vec![
                        Span::styled(
                            "Git Repository  ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(if val {
                            "Initialized"
                        } else {
                            "Not Initialized"
                        }),
                        if is_selected && !val {
                            Span::raw("  <[Enter] to initialize>")
                        } else {
                            Span::raw("")
                        },
                    ])
                }
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("⚙️ Settings"))
        .highlight_symbol("→ ");

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected_setting_index));

    f.render_stateful_widget(list, area, &mut state);
}
