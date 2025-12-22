use regex::Regex;
use std::sync::LazyLock;

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

// Regex for leading time-only timestamps that should be removed from log content
// Matches at start: 03:57:56, [14:30:45], (03:57:56.123), etc.
// Optional surrounding brackets/parens, trailing whitespace consumed
static LEADING_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\[\(\{]?\d{2}:\d{2}:\d{2}(?:\.\d+)?[\]\)\}]?\s*").unwrap()
});

/// Remove timestamps from log content and clean up surrounding whitespace.
/// Since we display arrival time in the log view, embedded timestamps are redundant.
/// Removes both ISO8601 timestamps and bare time-only timestamps (e.g., 03:57:56).
fn remove_timestamps(content: &str) -> String {
    let result = ISO8601_TIMESTAMP_REGEX.replace_all(content, "");
    // Remove leading time-only timestamp (only at start of line)
    let result = LEADING_TIME_REGEX.replace(&result, "");
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
    // First pass: remove timestamps (ISO8601 and bare time-only)
    let content = remove_timestamps(content);

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

/// Strip ANSI escape codes from a string
pub fn strip_ansi(content: &str) -> String {
    strip_ansi_escapes::strip_str(content).to_string()
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
    fn test_condense_removes_bare_leading_timestamp() {
        let input = "03:57:56 [DEBUG] (13) hyper::proto::h1::conn: incoming body";
        assert_eq!(condense_log_line(input), "[DEBUG] (13) hyper::proto::h1::conn: incoming body");
    }

    #[test]
    fn test_strip_ansi_removes_codes() {
        let input = "\x1b[31mRed text\x1b[0m";
        assert_eq!(strip_ansi(input), "Red text");
    }
}
