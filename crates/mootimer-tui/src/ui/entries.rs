use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw_entries(f: &mut Frame, app: &App, area: Rect) {
    let filtered_entries = app.get_filtered_entries();

    let entry_items: Vec<ListItem> = if filtered_entries.is_empty() {
        if !app.entry_filter.is_empty() {
            vec![
                ListItem::new(""),
                ListItem::new(format!("  No entries match filter: '{}'", app.entry_filter)),
                ListItem::new(""),
                ListItem::new("  Press [f] to change or clear filter."),
            ]
        } else {
            vec![
                ListItem::new(""),
                ListItem::new("  No entries for selected period."),
                ListItem::new(""),
                ListItem::new("  Press [d]/[w]/[m] to change time period."),
            ]
        }
    } else {
        filtered_entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let start_time_str = entry
                    .get("start_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let start_time_display =
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_time_str) {
                        dt.with_timezone(&chrono::Local)
                            .format("%H:%M:%S")
                            .to_string()
                    } else {
                        "--:--:--".to_string()
                    };

                let duration_secs = entry
                    .get("duration_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let hours = duration_secs / 3600;
                let minutes = (duration_secs % 3600) / 60;

                let task_title = entry.get("task_title").and_then(|v| v.as_str());
                let task_id = entry.get("task_id").and_then(|v| v.as_str());

                let task_display = if let Some(title) = task_title {
                    if let Some(tid) = task_id {
                        format!("{} [{}]", title, &tid[..8])
                    } else {
                        title.to_string()
                    }
                } else if let Some(tid) = task_id {
                    let task_name = app
                        .tasks
                        .iter()
                        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                        .and_then(|t| t.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown task");

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
                    "  {} {} {} │ {:>7} │ {}",
                    mode_icon,
                    if i == app.selected_entry_index {
                        "→"
                    } else {
                        " "
                    },
                    start_time_display,
                    time_str,
                    task_display
                );
                ListItem::new(text).style(style)
            })
            .collect()
    };

    let title = if app.entry_filter.is_empty() {
        format!(" 📝 Time Entries ({}) ", filtered_entries.len())
    } else {
        format!(
            " 📝 Time Entries ({} matched filter '{}') ",
            filtered_entries.len(),
            app.entry_filter
        )
    };

    let bottom_hint = " [d]ay [w]eek [m]onth | [e]dit [f]ilter [del]ete ";

    let entries_list = List::new(entry_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_bottom(Line::from(bottom_hint).right_aligned()),
    );
    f.render_widget(entries_list, area);
}
