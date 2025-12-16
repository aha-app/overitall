use anyhow::Result as AnyhowResult;
use arboard::Clipboard;

use crate::operations::logs::FilteredLogs;
use crate::process::ProcessManager;
use crate::ui::App;

fn copy_to_clipboard(text: &str) -> AnyhowResult<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Find a log line by its ID in the given list.
fn find_log_by_id<'a>(logs: &'a [crate::log::LogLine], id: u64) -> Option<&'a crate::log::LogLine> {
    logs.iter().find(|log| log.id == id)
}

/// Find the index of a log line by its ID in the given list.
fn find_index_by_id(logs: &[crate::log::LogLine], id: u64) -> Option<usize> {
    logs.iter().position(|log| log.id == id)
}

/// Copy the selected line to clipboard.
/// Returns Ok with success message or Err with error message.
pub fn copy_line(app: &App, manager: &ProcessManager) -> Result<String, String> {
    let line_id = app.selected_line_id
        .ok_or_else(|| "No line selected".to_string())?;

    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs = if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if !filtered.batches.is_empty() && batch_idx < filtered.batches.len() {
                let (start, end) = filtered.batches[batch_idx];
                filtered.logs[start..=end].to_vec()
            } else {
                filtered.logs
            }
        } else {
            filtered.logs
        }
    } else {
        filtered.logs
    };

    let log = find_log_by_id(&display_logs, line_id)
        .ok_or_else(|| "Selected line not found".to_string())?;

    let formatted = format!(
        "[{}] {}: {}",
        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
        log.source.process_name(),
        log.line
    );

    copy_to_clipboard(&formatted)
        .map(|_| "Copied line to clipboard".to_string())
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Format a slice of logs for clipboard output.
fn format_logs(logs: &[crate::log::LogLine]) -> String {
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

/// Copy the current context to clipboard (Shift+C).
/// Context-aware: copies trace, search results, or batch depending on current view.
/// Returns Ok with success message or Err with error message.
pub fn copy_context(app: &App, manager: &ProcessManager) -> Result<String, String> {
    // Get filtered logs
    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);

    // Priority 1: Trace mode - copy all trace lines
    if app.trace_filter_mode {
        return copy_trace(app, &filtered.logs);
    }

    // Priority 2: Search mode - copy all search results
    if !app.search_pattern.is_empty() {
        return copy_search_results(app, &filtered.logs);
    }

    // Priority 3: Default - copy the batch containing selected line
    copy_batch_internal(app, &filtered)
}

/// Copy all trace lines to clipboard.
fn copy_trace(app: &App, logs: &[crate::log::LogLine]) -> Result<String, String> {
    let trace_id = app.active_trace_id.as_ref()
        .ok_or_else(|| "No trace ID active".to_string())?;

    let (start, end) = match (app.trace_time_start, app.trace_time_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return Err("Trace time bounds not set".to_string()),
    };

    // Calculate expanded time window
    let expanded_start = start - app.trace_expand_before;
    let expanded_end = end + app.trace_expand_after;

    // Filter logs the same way log_viewer.rs does
    let trace_logs: Vec<_> = logs.iter()
        .filter(|log| {
            let contains_trace = log.line.contains(trace_id.as_str());
            let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
            contains_trace || (in_time_window && (app.trace_expand_before.num_seconds() > 0 || app.trace_expand_after.num_seconds() > 0))
        })
        .cloned()
        .collect();

    if trace_logs.is_empty() {
        return Err("No trace lines found".to_string());
    }

    let count = trace_logs.len();
    let mut text = format!("=== Trace: {} ({} lines) ===\n", trace_id, count);
    text.push_str(&format_logs(&trace_logs));

    copy_to_clipboard(&text)
        .map(|_| format!("Copied trace to clipboard ({} lines)", count))
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Copy all search results to clipboard.
fn copy_search_results(app: &App, logs: &[crate::log::LogLine]) -> Result<String, String> {
    let pattern = &app.search_pattern;
    let pattern_lower = pattern.to_lowercase();

    // Filter logs by search pattern (case-insensitive)
    let matching_logs: Vec<_> = logs.iter()
        .filter(|log| log.line.to_lowercase().contains(&pattern_lower))
        .cloned()
        .collect();

    if matching_logs.is_empty() {
        return Err("No search results to copy".to_string());
    }

    let count = matching_logs.len();
    let mut text = format!("=== Search: \"{}\" ({} matches) ===\n", pattern, count);
    text.push_str(&format_logs(&matching_logs));

    copy_to_clipboard(&text)
        .map(|_| format!("Copied search results to clipboard ({} matches)", count))
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Copy the batch containing the selected line to clipboard (internal helper).
fn copy_batch_internal(app: &App, filtered: &FilteredLogs) -> Result<String, String> {
    let line_id = app.selected_line_id
        .ok_or_else(|| "No line selected".to_string())?;

    // Find the line's index in the filtered logs
    let line_idx = find_index_by_id(&filtered.logs, line_id)
        .ok_or_else(|| "Selected line not found".to_string())?;

    // When in batch view mode, we're viewing a single batch
    let (batch_idx, start, end) = if app.batch_view_mode {
        if let Some(current_batch) = app.current_batch {
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
    let mut batch_text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, line_count);
    batch_text.push_str(&format_logs(&filtered.logs[start..=end]));

    copy_to_clipboard(&batch_text)
        .map(|_| format!("Copied batch to clipboard ({} lines)", line_count))
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Legacy function for backward compatibility - now calls copy_context.
pub fn copy_batch(app: &App, manager: &ProcessManager) -> Result<String, String> {
    copy_context(app, manager)
}
