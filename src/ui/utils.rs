use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
};
use ansi_to_tui::IntoText;

/// Helper function to create a centered rect using percentage of the available area
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Parse ANSI codes from text and optionally apply a background color override
pub fn parse_ansi_with_background(text: String, bg_color: Option<Color>, fg_override: Option<Color>) -> Line<'static> {
    // Try to parse ANSI codes
    match text.as_bytes().into_text() {
        Ok(parsed_text) => {
            // If we need to apply background or foreground override, modify all spans
            if bg_color.is_some() || fg_override.is_some() {
                let mut spans = Vec::new();
                for line in parsed_text.lines {
                    for span in line.spans {
                        let mut new_style = span.style;
                        if let Some(bg) = bg_color {
                            new_style = new_style.bg(bg);
                        }
                        if let Some(fg) = fg_override {
                            new_style = new_style.fg(fg);
                        }
                        spans.push(Span::styled(span.content, new_style));
                    }
                }
                Line::from(spans)
            } else {
                // No background override needed, use parsed text as-is
                // Convert Text to Line by taking the first line or combining all lines
                let mut spans = Vec::new();
                for line in parsed_text.lines {
                    spans.extend(line.spans);
                }
                Line::from(spans)
            }
        }
        Err(_) => {
            // Failed to parse ANSI, fall back to plain text
            let style = match (bg_color, fg_override) {
                (Some(bg), Some(fg)) => Style::default().bg(bg).fg(fg),
                (Some(bg), None) => Style::default().bg(bg),
                (None, Some(fg)) => Style::default().fg(fg),
                (None, None) => Style::default(),
            };
            Line::from(Span::styled(text.to_string(), style))
        }
    }
}
