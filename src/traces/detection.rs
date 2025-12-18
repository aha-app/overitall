use chrono::{DateTime, Local};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::log::LogLine;

/// A candidate trace (correlation ID) detected in the logs
#[derive(Debug, Clone)]
pub struct TraceCandidate {
    /// The token (e.g., UUID, long number) that appears to be a correlation ID
    pub token: String,
    /// First occurrence timestamp
    pub first_occurrence: DateTime<Local>,
    /// Last occurrence timestamp
    pub last_occurrence: DateTime<Local>,
    /// Number of log lines containing this token
    pub line_count: usize,
    /// A short context preview from the first occurrence
    pub context_preview: String,
}

// Regex patterns for correlation IDs
static UUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
        .unwrap()
});

static LONG_NUMERIC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{15,}\b").unwrap()
});

static LONG_HEX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{20,}\b").unwrap()
});

static LONG_ALPHANUMERIC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[a-zA-Z0-9]{16,}\b").unwrap()
});

/// Metadata for a token occurrence
struct TokenOccurrence {
    first_time: DateTime<Local>,
    last_time: DateTime<Local>,
    count: usize,
    first_line: String,
}

/// Extract potential correlation ID tokens from a log line
fn extract_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    // Extract UUIDs first (most specific pattern)
    for m in UUID_REGEX.find_iter(line) {
        tokens.push(m.as_str().to_string());
    }

    // Extract long numeric strings (15+ digits)
    for m in LONG_NUMERIC_REGEX.find_iter(line) {
        tokens.push(m.as_str().to_string());
    }

    // Extract long hex strings (20+ chars, skip if already matched as UUID)
    for m in LONG_HEX_REGEX.find_iter(line) {
        let s = m.as_str().to_string();
        // Skip if this looks like a UUID without dashes (we prefer the dashed version)
        if !tokens.contains(&s) && !s.chars().all(|c| c.is_ascii_hexdigit()) {
            tokens.push(s);
        }
    }

    // Extract long alphanumeric (16+ chars) - common for trace IDs
    for m in LONG_ALPHANUMERIC_REGEX.find_iter(line) {
        let s = m.as_str().to_string();
        if !tokens.contains(&s) {
            tokens.push(s);
        }
    }

    tokens
}

/// Check if a token passes the burstiness test (not a config value that appears everywhere)
fn is_bursty(occ: &TokenOccurrence, buffer_start: DateTime<Local>, buffer_end: DateTime<Local>) -> bool {
    let buffer_duration = (buffer_end - buffer_start).num_seconds() as f64;
    if buffer_duration <= 0.0 {
        return true; // Edge case: very short buffer, accept everything
    }

    let token_duration = (occ.last_time - occ.first_time).num_seconds() as f64;
    let span_ratio = token_duration / buffer_duration;

    // If the token spans more than 80% of the buffer time, it's probably a config value
    span_ratio < 0.8
}

/// Detect correlation ID traces in a set of log lines
/// Returns candidates sorted by first occurrence (most recent first)
pub fn detect_traces(logs: &[&LogLine]) -> Vec<TraceCandidate> {
    if logs.is_empty() {
        return Vec::new();
    }

    // Build token occurrence map
    let mut token_map: HashMap<String, TokenOccurrence> = HashMap::new();

    for log in logs {
        let tokens = extract_tokens(&log.line);
        for token in tokens {
            token_map
                .entry(token)
                .and_modify(|occ| {
                    if log.arrival_time < occ.first_time {
                        occ.first_time = log.arrival_time;
                        occ.first_line = log.line.clone();
                    }
                    if log.arrival_time > occ.last_time {
                        occ.last_time = log.arrival_time;
                    }
                    occ.count += 1;
                })
                .or_insert(TokenOccurrence {
                    first_time: log.arrival_time,
                    last_time: log.arrival_time,
                    count: 1,
                    first_line: log.line.clone(),
                });
        }
    }

    // Get buffer time range
    let buffer_start = logs.iter().map(|l| l.arrival_time).min().unwrap();
    let buffer_end = logs.iter().map(|l| l.arrival_time).max().unwrap();

    // Filter candidates: must appear 3+ times and pass burstiness check
    let mut candidates: Vec<TraceCandidate> = token_map
        .into_iter()
        .filter(|(_, occ)| occ.count >= 3 && is_bursty(occ, buffer_start, buffer_end))
        .map(|(token, occ)| {
            // Create a context preview (first 200 chars of first line, UI will truncate as needed)
            let preview = if occ.first_line.len() > 200 {
                format!("{}...", &occ.first_line[..200])
            } else {
                occ.first_line.clone()
            };

            TraceCandidate {
                token,
                first_occurrence: occ.first_time,
                last_occurrence: occ.last_time,
                line_count: occ.count,
                context_preview: preview,
            }
        })
        .collect();

    // Sort by first occurrence, most recent first
    candidates.sort_by(|a, b| b.first_occurrence.cmp(&a.first_occurrence));

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;
    use chrono::Duration;

    fn make_log(line: &str, arrival_offset_secs: i64) -> LogLine {
        let base_time = Local::now();
        let time = base_time + Duration::seconds(arrival_offset_secs);
        LogLine::new_with_time(LogSource::ProcessStdout("test".to_string()), line.to_string(), time)
    }

    #[test]
    fn test_extract_uuid() {
        let tokens = extract_tokens("Processing request 4ce710c3-a1b2-4c3d-8e5f-1234567890ab");
        assert!(tokens.contains(&"4ce710c3-a1b2-4c3d-8e5f-1234567890ab".to_string()));
    }

    #[test]
    fn test_extract_long_numeric() {
        let tokens = extract_tokens("Order ID: 123456789012345");
        assert!(tokens.contains(&"123456789012345".to_string()));
    }

    #[test]
    fn test_detect_traces_requires_3_occurrences() {
        let logs = vec![
            make_log("Request abc12345-1234-1234-1234-123456789012 started", 0),
            make_log("Processing abc12345-1234-1234-1234-123456789012", 1),
        ];
        let log_refs: Vec<&LogLine> = logs.iter().collect();
        let traces = detect_traces(&log_refs);
        // Only 2 occurrences, should not be detected
        assert!(traces.is_empty());
    }

    #[test]
    fn test_detect_traces_finds_uuid() {
        // Token spans 0-2 in a 0-100 buffer (2% span, well under 80% threshold)
        let logs = vec![
            make_log("Request abc12345-1234-1234-1234-123456789012 started", 0),
            make_log("Processing abc12345-1234-1234-1234-123456789012", 1),
            make_log("Completed abc12345-1234-1234-1234-123456789012", 2),
            make_log("Unrelated log line at end of buffer", 100),
        ];
        let log_refs: Vec<&LogLine> = logs.iter().collect();
        let traces = detect_traces(&log_refs);
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].token, "abc12345-1234-1234-1234-123456789012");
        assert_eq!(traces[0].line_count, 3);
    }

    #[test]
    fn test_burstiness_rejects_uniform_tokens() {
        // Token that appears across the entire buffer should be rejected
        let logs = vec![
            make_log("Config: abc12345-1234-1234-1234-123456789012", 0),
            make_log("Processing with abc12345-1234-1234-1234-123456789012", 100),
            make_log("Still using abc12345-1234-1234-1234-123456789012", 200),
        ];
        let log_refs: Vec<&LogLine> = logs.iter().collect();
        let traces = detect_traces(&log_refs);
        // Token spans 100% of the buffer (0 to 200), should be rejected
        assert!(traces.is_empty());
    }

    #[test]
    fn test_multiple_traces() {
        let logs = vec![
            make_log("Request aaa11111-1111-1111-1111-111111111111 started", 0),
            make_log("Processing aaa11111-1111-1111-1111-111111111111", 1),
            make_log("Completed aaa11111-1111-1111-1111-111111111111", 2),
            make_log("Request bbb22222-2222-2222-2222-222222222222 started", 10),
            make_log("Processing bbb22222-2222-2222-2222-222222222222", 11),
            make_log("Completed bbb22222-2222-2222-2222-222222222222", 12),
        ];
        let log_refs: Vec<&LogLine> = logs.iter().collect();
        let traces = detect_traces(&log_refs);
        assert_eq!(traces.len(), 2);
        // Most recent first
        assert_eq!(traces[0].token, "bbb22222-2222-2222-2222-222222222222");
        assert_eq!(traces[1].token, "aaa11111-1111-1111-1111-111111111111");
    }
}
