use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};
use ansi_to_tui::IntoText;

// Re-export condense_log_line from the log module for backwards compatibility
pub use crate::log::condense_log_line;

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

/// Parse ANSI codes from text and return spans with their styles (for caching)
pub fn parse_ansi_to_spans(text: &str) -> Vec<(String, Style)> {
    match text.as_bytes().into_text() {
        Ok(parsed_text) => {
            let mut spans = Vec::new();
            for line in parsed_text.lines {
                for span in line.spans {
                    spans.push((span.content.to_string(), span.style));
                }
            }
            spans
        }
        Err(_) => {
            vec![(text.to_string(), Style::default())]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condense_no_metadata() {
        let input = "Simple log message without metadata";
        assert_eq!(condense_log_line(input), input);
    }

    #[test]
    fn test_condense_single_metadata() {
        let input = "[user_id:123] Processing request";
        assert_eq!(condense_log_line(input), "[+1] Processing request");
    }

    #[test]
    fn test_condense_multiple_consecutive_metadata() {
        let input = "[user_id:0] [account_id:0] [request_uuid:abc123] Processing";
        assert_eq!(condense_log_line(input), "[+3] Processing");
    }

    #[test]
    fn test_condense_removes_leading_timestamp() {
        let input = "[23:47:16] web: [user_id:0] [account_id:0] Processing";
        assert_eq!(condense_log_line(input), "web: [+2] Processing");
    }

    #[test]
    fn test_condense_removes_leading_timestamp_with_millis() {
        let input = "[14:30:45.123] [pod:xyz] Message";
        assert_eq!(condense_log_line(input), "[+1] Message");
    }

    #[test]
    fn test_condense_non_consecutive_metadata() {
        let input = "[tag:a] some text [tag:b] more text";
        assert_eq!(condense_log_line(input), "[+1] some text [+1] more text");
    }

    #[test]
    fn test_condense_real_world_example() {
        let input = "[23:47:16] web: [user_id:0] [account_id:0] [request_uuid:web.2025-01-15] [pod:iad-dev1] Processing by Api::V1::ProjectsController#nav_pinned_features as JSON";
        let expected = "web: [+4] Processing by Api::V1::ProjectsController#nav_pinned_features as JSON";
        assert_eq!(condense_log_line(input), expected);
    }

    #[test]
    fn test_condense_empty_string() {
        assert_eq!(condense_log_line(""), "");
    }

    #[test]
    fn test_condense_removes_only_leading_timestamp() {
        let input = "[12:00:00] Just a timestamp and message";
        assert_eq!(condense_log_line(input), "Just a timestamp and message");
    }

    #[test]
    fn test_condense_removes_iso8601_with_timezone() {
        let input = "web: 2025-12-17T16:16:14+13:00 [WEB] POST /api/users - 500";
        assert_eq!(condense_log_line(input), "web: [WEB] POST /api/users - 500");
    }

    #[test]
    fn test_condense_removes_iso8601_with_z() {
        let input = "2025-12-17T16:16:14Z Processing request";
        assert_eq!(condense_log_line(input), "Processing request");
    }

    #[test]
    fn test_condense_removes_iso8601_with_milliseconds() {
        let input = "web: 2025-12-17T16:16:14.123+00:00 Starting up";
        assert_eq!(condense_log_line(input), "web: Starting up");
    }

    #[test]
    fn test_condense_removes_iso8601_without_timezone() {
        let input = "2025-12-17T16:16:14 Local time event";
        assert_eq!(condense_log_line(input), "Local time event");
    }

    #[test]
    fn test_condense_removes_iso8601_negative_offset() {
        let input = "2025-12-17T08:16:14-08:00 Pacific time";
        assert_eq!(condense_log_line(input), "Pacific time");
    }

    #[test]
    fn test_condense_combined_iso8601_and_metadata() {
        let input = "[23:47:16] web: 2025-12-17T23:47:16+13:00 [user_id:0] [account_id:0] Processing";
        assert_eq!(condense_log_line(input), "web: [+2] Processing");
    }

    #[test]
    fn test_condense_multiple_iso8601_timestamps() {
        let input = "Start: 2025-12-17T10:00:00Z End: 2025-12-17T11:00:00Z Done";
        assert_eq!(condense_log_line(input), "Start: End: Done");
    }
}
