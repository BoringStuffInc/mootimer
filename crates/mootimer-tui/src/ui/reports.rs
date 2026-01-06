use crate::app::App;
use crate::ui::helpers::format_duration_hm;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_reports(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(5)])
        .split(area);

    draw_report_summary(f, app, chunks[0]);
    draw_task_breakdown(f, app, chunks[1]);
}

fn draw_report_summary(f: &mut Frame, app: &App, area: Rect) {
    let report_text = if let Some(stats) = &app.report_stats {
        let total_secs = stats
            .get("total_duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
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

        let profile_label = if app.report_profile == "all" {
            "All Profiles".to_string()
        } else {
            app.get_profile_name_by_id(&app.report_profile).to_string()
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
            Line::from(format!(
                "  Total Time:        {}",
                format_duration_hm(total_secs)
            )),
            Line::from(format!(
                "  Total Sessions:    {}  (🍅 {} pomodoro, ⏱ {} manual)",
                entries, pomodoros, manual
            )),
            Line::from(format!("  Average Session:   {}m", avg_secs / 60)),
        ]
    } else {
        vec![Line::from(""), Line::from("  Loading...")]
    };

    let profile_label = if app.report_profile == "all" {
        "All Profiles"
    } else {
        app.get_profile_name_by_id(&app.report_profile)
    };

    let period_hint = "[d]ay [w]eek [m]onth";
    let profile_hint = "[p]rofile toggle";

    let report = Paragraph::new(report_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " 📈 {} Report - {} ",
                app.report_period.to_uppercase(),
                profile_label
            ))
            .title_bottom(
                Line::from(format!(" {} | {} | [r]efresh ", period_hint, profile_hint))
                    .right_aligned(),
            ),
    );
    f.render_widget(report, area);
}

fn draw_task_breakdown(f: &mut Frame, app: &App, area: Rect) {
    use std::collections::HashMap;

    let mut task_map: HashMap<String, (u64, usize)> = HashMap::new();

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

    let mut task_breakdown: Vec<(String, u64, usize)> = task_map
        .into_iter()
        .map(|(task_id, (secs, count))| (task_id, secs, count))
        .collect();

    task_breakdown.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

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

                format!("{} [{}]", task_name, &task_id[..8])
            };

            let time_str = if *total_secs >= 3600 {
                format_duration_hm(*total_secs)
            } else {
                format!("{}m", total_secs / 60)
            };

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
            .title(" 📋 Task Breakdown "),
    );
    f.render_widget(breakdown, area);
}
