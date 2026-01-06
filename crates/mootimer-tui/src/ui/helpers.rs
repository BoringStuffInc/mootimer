use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn focused_border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn build_hint_line<'a>(hint: &'a str, show_focus_hint: bool) -> Line<'a> {
    if show_focus_hint {
        Line::from(vec![
            Span::styled("[Ctrl+w]", Style::default().fg(Color::Yellow)),
            Span::raw("Focus "),
            Span::raw(hint),
        ])
    } else {
        Line::from(hint)
    }
}

pub fn format_duration_hms(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

pub fn format_duration_hm(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("{}h {:02}m", hours, minutes)
}
