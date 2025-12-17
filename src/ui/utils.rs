use regex::Regex;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};
use ansi_to_tui::IntoText;
use std::sync::LazyLock;

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

// Regex for matching bracketed key:value metadata patterns
// Matches: [key:value] where key is word chars/hyphens, value can be anything except ]
static BRACKET_METADATA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[[\w_-]+:[^\]]*\]").unwrap()
});

// Regex to detect timestamp-like brackets that should be preserved
// Matches: [HH:MM:SS] or [HH:MM:SS.mmm] format
static TIMESTAMP_BRACKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[\d{1,2}:\d{2}:\d{2}(?:\.\d+)?\]$").unwrap()
});

// Regex for ISO8601 timestamps that should be removed from log content
// Matches: 2025-12-17T16:16:14+13:00, 2025-12-17T16:16:14.123+13:00, 2025-12-17T16:16:14Z, etc.
static ISO8601_TIMESTAMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?"
    ).unwrap()
});

/// Remove ISO8601 timestamps from log content and clean up surrounding whitespace.
/// Since we display arrival time in the log view, embedded timestamps are redundant.
fn remove_iso8601_timestamps(content: &str) -> String {
    let result = ISO8601_TIMESTAMP_REGEX.replace_all(content, "");
    // Clean up any double spaces left behind and trim leading/trailing spaces
    let mut cleaned = String::with_capacity(result.len());
    let mut prev_was_space = true; // Start true to skip leading spaces
    for ch in result.chars() {
        if ch == ' ' {
            if !prev_was_space {
                cleaned.push(ch);
            }
            prev_was_space = true;
        } else {
            cleaned.push(ch);
            prev_was_space = false;
        }
    }
    // Trim trailing space if present
    if cleaned.ends_with(' ') {
        cleaned.pop();
    }
    cleaned
}

/// Condense log line content by:
/// 1. Removing ISO8601 timestamps (e.g., 2025-12-17T16:16:14+13:00) since we display arrival time
/// 2. Collapsing consecutive [key:value] metadata tags into a single [+N] indicator
///
/// Example:
///   Input:  "[23:47:16] web: 2025-12-17T16:16:14+13:00 [user_id:0] Processing..."
///   Output: "[23:47:16] web: [+1] Processing..."
pub fn condense_log_line(content: &str) -> String {
    // First pass: remove ISO8601 timestamps
    let content = remove_iso8601_timestamps(content);

    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;
    let mut pending_metadata_count = 0;
    let mut pending_metadata_start: Option<usize> = None;

    for mat in BRACKET_METADATA_REGEX.find_iter(&content) {
        let matched_text = mat.as_str();

        // Check if this looks like a timestamp - preserve it
        if TIMESTAMP_BRACKET_REGEX.is_match(matched_text) {
            // Flush any pending collapsed metadata first
            if pending_metadata_count > 0 {
                result.push_str(&format!("[+{}]", pending_metadata_count));
                pending_metadata_count = 0;
                pending_metadata_start = None;
            }
            // Add content before this match (if any gap)
            if mat.start() > last_end {
                result.push_str(&content[last_end..mat.start()]);
            }
            // Preserve the timestamp
            result.push_str(matched_text);
            last_end = mat.end();
            continue;
        }

        // This is a key:value metadata bracket - should be collapsed
        if pending_metadata_start.is_none() {
            // First metadata bracket in a sequence
            // Add any content before this match
            if mat.start() > last_end {
                result.push_str(&content[last_end..mat.start()]);
            }
            pending_metadata_start = Some(mat.start());
            pending_metadata_count = 1;
        } else {
            // Check if this is consecutive (only whitespace between)
            let gap = &content[last_end..mat.start()];
            if gap.chars().all(|c| c.is_whitespace()) {
                // Consecutive metadata, add to count
                pending_metadata_count += 1;
            } else {
                // Not consecutive - flush pending and start new sequence
                if pending_metadata_count > 0 {
                    result.push_str(&format!("[+{}]", pending_metadata_count));
                }
                result.push_str(gap);
                pending_metadata_start = Some(mat.start());
                pending_metadata_count = 1;
            }
        }
        last_end = mat.end();
    }

    // Flush any remaining pending metadata
    if pending_metadata_count > 0 {
        result.push_str(&format!("[+{}]", pending_metadata_count));
    }

    // Add remaining content after last match
    if last_end < content.len() {
        result.push_str(&content[last_end..]);
    }

    // If no matches were found, return original content
    if result.is_empty() && pending_metadata_count == 0 {
        return content.to_string();
    }

    result
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
    fn test_condense_preserves_timestamp() {
        let input = "[23:47:16] web: [user_id:0] [account_id:0] Processing";
        assert_eq!(condense_log_line(input), "[23:47:16] web: [+2] Processing");
    }

    #[test]
    fn test_condense_preserves_timestamp_with_millis() {
        let input = "[14:30:45.123] [pod:xyz] Message";
        assert_eq!(condense_log_line(input), "[14:30:45.123] [+1] Message");
    }

    #[test]
    fn test_condense_non_consecutive_metadata() {
        let input = "[tag:a] some text [tag:b] more text";
        assert_eq!(condense_log_line(input), "[+1] some text [+1] more text");
    }

    #[test]
    fn test_condense_real_world_example() {
        let input = "[23:47:16] web: [user_id:0] [account_id:0] [request_uuid:web.2025-01-15] [pod:iad-dev1] Processing by Api::V1::ProjectsController#nav_pinned_features as JSON";
        let expected = "[23:47:16] web: [+4] Processing by Api::V1::ProjectsController#nav_pinned_features as JSON";
        assert_eq!(condense_log_line(input), expected);
    }

    #[test]
    fn test_condense_empty_string() {
        assert_eq!(condense_log_line(""), "");
    }

    #[test]
    fn test_condense_only_timestamp() {
        let input = "[12:00:00] Just a timestamp and message";
        assert_eq!(condense_log_line(input), input);
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
        assert_eq!(condense_log_line(input), "[23:47:16] web: [+2] Processing");
    }

    #[test]
    fn test_condense_multiple_iso8601_timestamps() {
        let input = "Start: 2025-12-17T10:00:00Z End: 2025-12-17T11:00:00Z Done";
        assert_eq!(condense_log_line(input), "Start: End: Done");
    }

    #[test]
    fn test_remove_iso8601_cleans_double_spaces() {
        let result = remove_iso8601_timestamps("before 2025-12-17T10:00:00Z after");
        assert_eq!(result, "before after");
    }
}
