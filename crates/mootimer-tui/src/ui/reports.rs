use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

pub fn draw_reports(f: &mut Frame, app: &App, area: Rect) {
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

    // Sort by duration (descending), then by task_id (ascending) for stable sort
    task_breakdown.sort_by(|a, b| {
        b.1.cmp(&a.1) // Sort by duration descending
            .then_with(|| a.0.cmp(&b.0)) // Then by task_id ascending
    });

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
