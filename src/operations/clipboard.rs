use anyhow::Result as AnyhowResult;
use arboard::Clipboard;

use crate::log::LogLine;
use crate::operations::logs::FilteredLogs;
use crate::operations::navigation::get_selected_log_ids;
use crate::process::ProcessManager;
use crate::ui::App;

fn copy_to_clipboard(text: &str) -> AnyhowResult<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Find a log line by its ID in the given list.
fn find_log_by_id<'a>(logs: &'a [LogLine], id: u64) -> Option<&'a LogLine> {
    logs.iter().find(|log| log.id == id)
}

/// Find the index of a log line by its ID in the given list.
fn find_index_by_id(logs: &[LogLine], id: u64) -> Option<usize> {
    logs.iter().position(|log| log.id == id)
}

/// Format a slice of logs for clipboard output.
fn format_logs(logs: &[LogLine]) -> String {
    let mut text = String::new();
    for log in logs {
        text.push_str(&format!(
            "[{}] {}: {}\n",
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.source.process_name(),
            log.line
        ));
    }
    text
}

/// Represents what should be copied and the success message.
#[derive(Debug)]
pub struct CopyResult {
    pub text: String,
    pub message: String,
}

/// Determines which copy mode should be used based on app state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMode {
    Trace,
    Batch,
    Search,
}

/// Determine the copy mode based on current app state.
pub fn determine_copy_mode(app: &App) -> CopyMode {
    if app.trace.trace_filter_mode {
        CopyMode::Trace
    } else if app.batch.batch_view_mode {
        CopyMode::Batch
    } else if !app.input.search_pattern.is_empty() {
        CopyMode::Search
    } else {
        CopyMode::Batch
    }
}

/// Build the text for copying a single line.
pub fn build_line_text(app: &App, filtered: &FilteredLogs) -> Result<CopyResult, String> {
    let line_id = app.navigation.selected_line_id
        .ok_or_else(|| "No line selected".to_string())?;

    // Apply batch view mode filtering if enabled
    let display_logs = if app.batch.batch_view_mode {
        if let Some(batch_idx) = app.batch.current_batch {
            if !filtered.batches.is_empty() && batch_idx < filtered.batches.len() {
                let (start, end) = filtered.batches[batch_idx];
                filtered.logs[start..=end].to_vec()
            } else {
                filtered.logs.clone()
            }
        } else {
            filtered.logs.clone()
        }
    } else {
        filtered.logs.clone()
    };

    let log = find_log_by_id(&display_logs, line_id)
        .ok_or_else(|| "Selected line not found".to_string())?;

    let text = format!(
        "[{}] {}: {}",
        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
        log.source.process_name(),
        log.line
    );

    Ok(CopyResult {
        text,
        message: "Copied line to clipboard".to_string(),
    })
}

/// Build the text for copying multiple selected lines.
pub fn build_multi_select_text(app: &App, filtered: &FilteredLogs) -> Result<CopyResult, String> {
    // Apply batch view mode filtering if enabled
    let display_logs = if app.batch.batch_view_mode {
        if let Some(batch_idx) = app.batch.current_batch {
            if !filtered.batches.is_empty() && batch_idx < filtered.batches.len() {
                let (start, end) = filtered.batches[batch_idx];
                filtered.logs[start..=end].to_vec()
            } else {
                filtered.logs.clone()
            }
        } else {
            filtered.logs.clone()
        }
    } else {
        filtered.logs.clone()
    };

    let selected_ids = get_selected_log_ids(app, &display_logs);

    if selected_ids.is_empty() {
        return Err("No lines selected".to_string());
    }

    // Collect matching logs in display order
    let selected_logs: Vec<LogLine> = display_logs
        .iter()
        .filter(|log| selected_ids.contains(&log.id))
        .cloned()
        .collect();

    let count = selected_logs.len();
    let text = format_logs(&selected_logs);

    Ok(CopyResult {
        text,
        message: format!("Copied {} lines", count),
    })
}

/// Build the text for copying trace lines.
pub fn build_trace_text(app: &App, logs: &[LogLine]) -> Result<CopyResult, String> {
    let trace_id = app.trace.active_trace_id.as_ref()
        .ok_or_else(|| "No trace ID active".to_string())?;

    let (start, end) = match (app.trace.trace_time_start, app.trace.trace_time_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return Err("Trace time bounds not set".to_string()),
    };

    // Calculate expanded time window
    let expanded_start = start - app.trace.trace_expand_before;
    let expanded_end = end + app.trace.trace_expand_after;

    // Filter logs the same way log_viewer.rs does
    let trace_logs: Vec<_> = logs.iter()
        .filter(|log| {
            let contains_trace = log.line.contains(trace_id.as_str());
            let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
            contains_trace || (in_time_window && (app.trace.trace_expand_before.num_seconds() > 0 || app.trace.trace_expand_after.num_seconds() > 0))
        })
        .cloned()
        .collect();

    if trace_logs.is_empty() {
        return Err("No trace lines found".to_string());
    }

    let count = trace_logs.len();
    let mut text = format!("=== Trace: {} ({} lines) ===\n", trace_id, count);
    text.push_str(&format_logs(&trace_logs));

    Ok(CopyResult {
        text,
        message: format!("Copied trace to clipboard ({} lines)", count),
    })
}

/// Build the text for copying search results.
pub fn build_search_text(app: &App, logs: &[LogLine]) -> Result<CopyResult, String> {
    let pattern = &app.input.search_pattern;
    let pattern_lower = pattern.to_lowercase();

    // Filter logs by search pattern (case-insensitive)
    let matching_logs: Vec<_> = logs.iter()
        .filter(|log| log.line_lowercase().contains(&pattern_lower))
        .cloned()
        .collect();

    if matching_logs.is_empty() {
        return Err("No search results to copy".to_string());
    }

    let count = matching_logs.len();
    let mut text = format!("=== Search: \"{}\" ({} matches) ===\n", pattern, count);
    text.push_str(&format_logs(&matching_logs));

    Ok(CopyResult {
        text,
        message: format!("Copied search results to clipboard ({} matches)", count),
    })
}

/// Build the text for copying a batch.
pub fn build_batch_text(app: &App, filtered: &FilteredLogs) -> Result<CopyResult, String> {
    let line_id = app.navigation.selected_line_id
        .ok_or_else(|| "No line selected".to_string())?;

    // Find the line's index in the filtered logs
    let line_idx = find_index_by_id(&filtered.logs, line_id)
        .ok_or_else(|| "Selected line not found".to_string())?;

    // When in batch view mode, we're viewing a single batch
    let (batch_idx, start, end) = if app.batch.batch_view_mode {
        if let Some(current_batch) = app.batch.current_batch {
            if current_batch < filtered.batches.len() {
                let (s, e) = filtered.batches[current_batch];
                (current_batch, s, e)
            } else {
                return Err("Current batch out of range".to_string());
            }
        } else {
            return Err("No batch selected".to_string());
        }
    } else {
        // Not in batch view mode - find which batch contains the selected line
        filtered.batches.iter().enumerate()
            .find(|(_, (start, end))| line_idx >= *start && line_idx <= *end)
            .map(|(idx, (s, e))| (idx, *s, *e))
            .ok_or_else(|| "No batch found for selected line".to_string())?
    };

    // Format the entire batch
    let line_count = end - start + 1;
    let mut text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, line_count);
    text.push_str(&format_logs(&filtered.logs[start..=end]));

    Ok(CopyResult {
        text,
        message: format!("Copied batch to clipboard ({} lines)", line_count),
    })
}

/// Build the context-aware copy text based on current app state.
pub fn build_context_text(app: &App, filtered: &FilteredLogs) -> Result<CopyResult, String> {
    match determine_copy_mode(app) {
        CopyMode::Trace => build_trace_text(app, &filtered.logs),
        CopyMode::Batch => build_batch_text(app, filtered),
        CopyMode::Search => build_search_text(app, &filtered.logs),
    }
}

/// Copy the selected line(s) to clipboard.
/// If multi-select is active, copies all selected lines.
/// Returns Ok with success message or Err with error message.
pub fn copy_line(app: &App, manager: &ProcessManager) -> Result<String, String> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, app.batch.batch_window_ms);

    let result = if app.navigation.has_multi_select() {
        build_multi_select_text(app, &filtered)?
    } else {
        build_line_text(app, &filtered)?
    };

    copy_to_clipboard(&result.text)
        .map(|_| result.message)
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Copy the current context to clipboard (Shift+C).
/// Context-aware: copies trace, search results, or batch depending on current view.
/// Returns Ok with success message or Err with error message.
pub fn copy_context(app: &App, manager: &ProcessManager) -> Result<String, String> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, app.batch.batch_window_ms);
    let result = build_context_text(app, &filtered)?;

    copy_to_clipboard(&result.text)
        .map(|_| result.message)
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Legacy function for backward compatibility - now calls copy_context.
pub fn copy_batch(app: &App, manager: &ProcessManager) -> Result<String, String> {
    copy_context(app, manager)
}

/// Build text for copying logs from the same process within a time window around the selected line.
pub fn build_time_context_text(
    app: &App,
    filtered: &FilteredLogs,
    time_window_seconds: f64,
) -> Result<CopyResult, String> {
    let line_id = app
        .navigation
        .selected_line_id
        .ok_or_else(|| "No line selected".to_string())?;

    // Find the selected log
    let selected_log = find_log_by_id(&filtered.logs, line_id)
        .ok_or_else(|| "Selected line not found".to_string())?;

    let process_name = selected_log.source.process_name().to_string();
    let center_time = selected_log.arrival_time;
    let window = chrono::Duration::milliseconds((time_window_seconds * 1000.0) as i64);

    // Filter logs: same process AND within time window
    let context_logs: Vec<_> = filtered
        .logs
        .iter()
        .filter(|log| {
            log.source.process_name() == process_name
                && log.arrival_time >= center_time - window
                && log.arrival_time <= center_time + window
        })
        .cloned()
        .collect();

    if context_logs.is_empty() {
        return Err("No context lines found".to_string());
    }

    let count = context_logs.len();
    let text = format_logs(&context_logs);

    Ok(CopyResult {
        text,
        message: format!("Contextual copy: {} lines ({} ±{}s)", count, process_name, time_window_seconds),
    })
}

/// Copy logs from the same process within a time window around the selected line (Ctrl+Shift+C).
/// Returns Ok with success message or Err with error message.
pub fn copy_time_context(
    app: &App,
    manager: &ProcessManager,
    time_window_seconds: f64,
) -> Result<String, String> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, app.batch.batch_window_ms);
    let result = build_time_context_text(app, &filtered, time_window_seconds)?;

    copy_to_clipboard(&result.text)
        .map(|_| result.message)
        .map_err(|e| format!("Failed to copy: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;
    use chrono::{Duration, Local};

    fn create_test_logs() -> Vec<LogLine> {
        let now = Local::now();
        vec![
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Starting server".to_string(),
                now,
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "ERROR: Connection failed".to_string(),
                now + Duration::milliseconds(10),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("worker".to_string()),
                "Processing job trace-abc123".to_string(),
                now + Duration::milliseconds(20),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("worker".to_string()),
                "ERROR: Job failed trace-abc123".to_string(),
                now + Duration::milliseconds(500),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Request completed".to_string(),
                now + Duration::milliseconds(510),
            ),
        ]
    }

    fn create_filtered_logs(logs: Vec<LogLine>) -> FilteredLogs {
        // Create batches based on 100ms window
        // Logs 1,2,3 are within 100ms -> batch 0
        // Logs 4,5 are within 100ms -> batch 1
        FilteredLogs {
            logs,
            batches: vec![(0, 2), (3, 4)],
        }
    }

    #[test]
    fn test_determine_copy_mode_trace() {
        let mut app = App::new();
        app.trace.trace_filter_mode = true;

        assert_eq!(determine_copy_mode(&app), CopyMode::Trace);
    }

    #[test]
    fn test_determine_copy_mode_batch_view() {
        let mut app = App::new();
        app.batch.batch_view_mode = true;

        assert_eq!(determine_copy_mode(&app), CopyMode::Batch);
    }

    #[test]
    fn test_determine_copy_mode_search() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();

        assert_eq!(determine_copy_mode(&app), CopyMode::Search);
    }

    #[test]
    fn test_determine_copy_mode_batch_view_overrides_search() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();
        app.batch.batch_view_mode = true;

        // Batch view mode should take priority over search
        assert_eq!(determine_copy_mode(&app), CopyMode::Batch);
    }

    #[test]
    fn test_determine_copy_mode_trace_overrides_all() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();
        app.batch.batch_view_mode = true;
        app.trace.trace_filter_mode = true;

        // Trace mode should take priority over everything
        assert_eq!(determine_copy_mode(&app), CopyMode::Trace);
    }

    #[test]
    fn test_determine_copy_mode_default_is_batch() {
        let app = App::new();

        assert_eq!(determine_copy_mode(&app), CopyMode::Batch);
    }

    #[test]
    fn test_build_search_text_filters_correctly() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();

        let logs = create_test_logs();
        let result = build_search_text(&app, &logs).unwrap();

        // Should find 2 ERROR lines
        assert!(result.text.contains("=== Search: \"ERROR\" (2 matches) ==="));
        assert!(result.text.contains("ERROR: Connection failed"));
        assert!(result.text.contains("ERROR: Job failed"));
        assert!(!result.text.contains("Starting server"));
        assert!(!result.text.contains("Request completed"));
        assert_eq!(result.message, "Copied search results to clipboard (2 matches)");
    }

    #[test]
    fn test_build_search_text_case_insensitive() {
        let mut app = App::new();
        app.input.search_pattern = "error".to_string(); // lowercase

        let logs = create_test_logs();
        let result = build_search_text(&app, &logs).unwrap();

        // Should still find ERROR lines (case-insensitive)
        assert!(result.text.contains("(2 matches)"));
    }

    #[test]
    fn test_build_search_text_no_matches() {
        let mut app = App::new();
        app.input.search_pattern = "NONEXISTENT".to_string();

        let logs = create_test_logs();
        let result = build_search_text(&app, &logs);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No search results to copy");
    }

    #[test]
    fn test_build_batch_text_in_batch_view_mode() {
        let mut app = App::new();
        app.batch.batch_view_mode = true;
        app.batch.current_batch = Some(0);

        let logs = create_test_logs();
        // Use the first log's actual ID
        app.navigation.selected_line_id = Some(logs[0].id);
        let filtered = create_filtered_logs(logs);

        let result = build_batch_text(&app, &filtered).unwrap();

        // Batch 0 has 3 lines (indices 0-2)
        assert!(result.text.contains("=== Batch 1 (3 lines) ==="));
        assert!(result.text.contains("Starting server"));
        assert!(result.text.contains("ERROR: Connection failed"));
        assert!(result.text.contains("Processing job trace-abc123"));
        assert!(!result.text.contains("ERROR: Job failed")); // batch 1
        assert_eq!(result.message, "Copied batch to clipboard (3 lines)");
    }

    #[test]
    fn test_build_batch_text_second_batch() {
        let mut app = App::new();
        app.batch.batch_view_mode = true;
        app.batch.current_batch = Some(1);

        let logs = create_test_logs();
        // Use the fourth log's actual ID (index 3)
        app.navigation.selected_line_id = Some(logs[3].id);
        let filtered = create_filtered_logs(logs);

        let result = build_batch_text(&app, &filtered).unwrap();

        // Batch 1 has 2 lines (indices 3-4)
        assert!(result.text.contains("=== Batch 2 (2 lines) ==="));
        assert!(result.text.contains("ERROR: Job failed"));
        assert!(result.text.contains("Request completed"));
        assert!(!result.text.contains("Starting server")); // batch 0
        assert_eq!(result.message, "Copied batch to clipboard (2 lines)");
    }

    #[test]
    fn test_build_batch_text_no_selection() {
        let mut app = App::new();
        app.batch.batch_view_mode = true;
        app.batch.current_batch = Some(0);
        // No selected_line_id

        let logs = create_test_logs();
        let filtered = create_filtered_logs(logs);

        let result = build_batch_text(&app, &filtered);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No line selected");
    }

    #[test]
    fn test_build_context_text_uses_batch_when_in_batch_view_with_search() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();
        app.batch.batch_view_mode = true;
        app.batch.current_batch = Some(0);

        let logs = create_test_logs();
        // Use the first log's actual ID
        app.navigation.selected_line_id = Some(logs[0].id);
        let filtered = create_filtered_logs(logs);

        let result = build_context_text(&app, &filtered).unwrap();

        // Should copy batch, not search results
        assert!(result.text.contains("=== Batch 1"));
        assert!(!result.text.contains("=== Search:"));
    }

    #[test]
    fn test_build_context_text_uses_search_when_not_in_batch_view() {
        let mut app = App::new();
        app.input.search_pattern = "ERROR".to_string();
        // batch_view_mode is false by default
        // Need a selection for batch mode fallback, but search takes priority

        let logs = create_test_logs();
        let filtered = create_filtered_logs(logs);

        let result = build_context_text(&app, &filtered).unwrap();

        // Should copy search results
        assert!(result.text.contains("=== Search: \"ERROR\""));
        assert!(!result.text.contains("=== Batch"));
    }

    #[test]
    fn test_build_line_text() {
        let mut app = App::new();

        let logs = create_test_logs();
        // Use the second log's actual ID (index 1, contains "ERROR: Connection failed")
        app.navigation.selected_line_id = Some(logs[1].id);
        let filtered = create_filtered_logs(logs);

        let result = build_line_text(&app, &filtered).unwrap();

        assert!(result.text.contains("web: ERROR: Connection failed"));
        assert_eq!(result.message, "Copied line to clipboard");
    }

    #[test]
    fn test_build_trace_text() {
        let mut app = App::new();
        let now = Local::now();

        app.trace.trace_filter_mode = true;
        app.trace.active_trace_id = Some("trace-abc123".to_string());
        app.trace.trace_time_start = Some(now);
        app.trace.trace_time_end = Some(now + Duration::seconds(1));

        let logs = create_test_logs();

        let result = build_trace_text(&app, &logs).unwrap();

        assert!(result.text.contains("=== Trace: trace-abc123 (2 lines) ==="));
        assert!(result.text.contains("Processing job trace-abc123"));
        assert!(result.text.contains("ERROR: Job failed trace-abc123"));
        assert!(!result.text.contains("Starting server"));
        assert_eq!(result.message, "Copied trace to clipboard (2 lines)");
    }

    #[test]
    fn test_build_time_context_text_filters_by_process_and_time() {
        let mut app = App::new();
        let now = Local::now();

        // Create logs with specific timing for testing
        let logs = vec![
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Web log 1".to_string(),
                now,
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Web log 2".to_string(),
                now + Duration::milliseconds(500),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("worker".to_string()),
                "Worker log 1".to_string(),
                now + Duration::milliseconds(600),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Web log 3".to_string(),
                now + Duration::milliseconds(800),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Web log 4 outside window".to_string(),
                now + Duration::milliseconds(2500),
            ),
        ];

        // Select the second web log
        app.navigation.selected_line_id = Some(logs[1].id);

        let filtered = FilteredLogs {
            logs: logs.clone(),
            batches: vec![(0, 4)],
        };

        // Use 1 second window
        let result = build_time_context_text(&app, &filtered, 1.0).unwrap();

        // Should include web logs within ±1s of log[1] (which is at +500ms)
        // Window: -500ms to +1500ms from start
        // Log 0: 0ms - YES (within window)
        // Log 1: 500ms - YES (the selected one)
        // Log 2: 600ms - NO (wrong process)
        // Log 3: 800ms - YES (within window)
        // Log 4: 2500ms - NO (outside window, +2000ms from selected)
        assert!(result.text.contains("Web log 1"));
        assert!(result.text.contains("Web log 2"));
        assert!(result.text.contains("Web log 3"));
        assert!(!result.text.contains("Worker log 1")); // different process
        assert!(!result.text.contains("Web log 4")); // outside time window
        assert_eq!(result.message, "Contextual copy: 3 lines (web ±1s)");
    }

    #[test]
    fn test_build_time_context_text_no_selection() {
        let app = App::new();
        let logs = create_test_logs();
        let filtered = create_filtered_logs(logs);

        let result = build_time_context_text(&app, &filtered, 1.0);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No line selected");
    }

    #[test]
    fn test_build_time_context_text_selected_line_not_found() {
        let mut app = App::new();
        app.navigation.selected_line_id = Some(99999); // Non-existent ID

        let logs = create_test_logs();
        let filtered = create_filtered_logs(logs);

        let result = build_time_context_text(&app, &filtered, 1.0);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Selected line not found");
    }

    #[test]
    fn test_build_time_context_text_small_window() {
        let mut app = App::new();
        let now = Local::now();

        let logs = vec![
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Early log".to_string(),
                now,
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Selected log".to_string(),
                now + Duration::milliseconds(1000),
            ),
            LogLine::new_with_time(
                LogSource::ProcessStdout("web".to_string()),
                "Late log".to_string(),
                now + Duration::milliseconds(2000),
            ),
        ];

        app.navigation.selected_line_id = Some(logs[1].id);

        let filtered = FilteredLogs {
            logs: logs.clone(),
            batches: vec![(0, 2)],
        };

        // Use 0.5 second window - only the selected log should be included
        let result = build_time_context_text(&app, &filtered, 0.5).unwrap();

        assert!(result.text.contains("Selected log"));
        assert!(!result.text.contains("Early log")); // 1000ms before, outside ±500ms
        assert!(!result.text.contains("Late log")); // 1000ms after, outside ±500ms
        assert_eq!(result.message, "Contextual copy: 1 lines (web ±0.5s)");
    }
}
