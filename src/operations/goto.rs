use crate::command::GotoTarget;
use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::{App, FilterType, detect_batches_from_logs};
use chrono::NaiveTime;

/// Get the list of logs to display based on current view mode.
/// This matches the filtering logic in log_viewer.rs exactly.
fn get_display_logs(app: &App, manager: &ProcessManager) -> Vec<LogLine> {
    // Use snapshot if available (frozen/batch mode), otherwise use live buffer
    let logs_vec: Vec<&LogLine> = if let Some(ref snapshot) = app.snapshot {
        snapshot.iter().collect()
    } else {
        let mut logs = manager.get_all_logs();

        // If display is frozen (without snapshot), only show logs up to the frozen timestamp
        if app.frozen {
            if let Some(frozen_at) = app.frozen_at {
                logs.retain(|log| log.timestamp <= frozen_at);
            }
        }

        logs
    };

    // Apply filters to logs
    let mut filtered_logs: Vec<&LogLine> = if app.filters.is_empty() {
        logs_vec
    } else {
        logs_vec.into_iter()
            .filter(|log| {
                let line_text = &log.line;

                // Check exclude filters first (if any match, reject the line)
                for filter in &app.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches(line_text) {
                            return false;
                        }
                    }
                }

                // Check include filters (if any exist, at least one must match)
                let include_filters: Vec<_> = app
                    .filters
                    .iter()
                    .filter(|f| matches!(f.filter_type, FilterType::Include))
                    .collect();

                if include_filters.is_empty() {
                    return true;
                }

                include_filters.iter().any(|filter| filter.matches(line_text))
            })
            .collect()
    };

    // Apply search filter if active
    let active_search_pattern = if app.search_mode && !app.input.is_empty() {
        &app.input
    } else if !app.search_pattern.is_empty() {
        &app.search_pattern
    } else {
        ""
    };

    if !active_search_pattern.is_empty() {
        let pattern_lower = active_search_pattern.to_lowercase();
        filtered_logs = filtered_logs
            .into_iter()
            .filter(|log| log.line_lowercase().contains(&pattern_lower))
            .collect();
    }

    // Apply process visibility filter
    filtered_logs.retain(|log| {
        !app.hidden_processes.contains(log.source.process_name())
    });

    // Apply trace filter mode if active
    if app.trace_filter_mode {
        if let (Some(trace_id), Some(start), Some(end)) = (
            &app.active_trace_id,
            app.trace_time_start,
            app.trace_time_end,
        ) {
            let expanded_start = start - app.trace_expand_before;
            let expanded_end = end + app.trace_expand_after;

            filtered_logs = filtered_logs
                .into_iter()
                .filter(|log| {
                    let contains_trace = log.line.contains(trace_id.as_str());
                    let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
                    contains_trace || (in_time_window && (app.trace_expand_before.num_seconds() > 0 || app.trace_expand_after.num_seconds() > 0))
                })
                .collect();
        }
    }

    // Detect batches from filtered logs
    let batches = detect_batches_from_logs(&filtered_logs, app.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs: Vec<LogLine> = if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if !batches.is_empty() && batch_idx < batches.len() {
                let (start, end) = batches[batch_idx];
                filtered_logs[start..=end].iter().map(|l| (*l).clone()).collect()
            } else {
                filtered_logs.into_iter().cloned().collect()
            }
        } else {
            filtered_logs.into_iter().cloned().collect()
        }
    } else {
        filtered_logs.into_iter().cloned().collect()
    };

    display_logs
}

/// Navigate to a specific timestamp in the log view.
/// Returns Ok with a status message, or Err with an error message.
pub fn goto_timestamp(app: &mut App, manager: &ProcessManager, target: GotoTarget) -> Result<String, String> {
    let display_logs = get_display_logs(app, manager);

    if display_logs.is_empty() {
        return Err("No logs to navigate".to_string());
    }

    let target_idx = match target {
        GotoTarget::AbsoluteTime { hour, minute, second } => {
            find_by_absolute_time(&display_logs, hour, minute, second)
        }
        GotoTarget::RelativeTime { seconds } => {
            find_by_relative_time(app, &display_logs, seconds)
        }
    };

    match target_idx {
        Some(idx) => {
            // Create snapshot if not already frozen
            if app.snapshot.is_none() {
                let logs = manager.get_all_logs();
                let filtered = crate::ui::apply_filters(logs, &app.filters);
                app.create_snapshot(filtered);
            }

            let log = &display_logs[idx];
            app.selected_line_id = Some(log.id);
            app.auto_scroll = false;
            app.freeze_display();

            let time_str = log.timestamp.format("%H:%M:%S").to_string();
            Ok(format!("Jumped to {}", time_str))
        }
        None => Err("No log found at target time".to_string()),
    }
}

/// Find the first log at or after the specified absolute time.
fn find_by_absolute_time(logs: &[LogLine], hour: u32, minute: u32, second: Option<u32>) -> Option<usize> {
    let target_time = NaiveTime::from_hms_opt(hour, minute, second.unwrap_or(0))?;

    // Find the first log whose time is >= target time
    for (idx, log) in logs.iter().enumerate() {
        let log_time = log.timestamp.time();
        if log_time >= target_time {
            return Some(idx);
        }
    }

    // If no log is at or after target time, return the last log
    if !logs.is_empty() {
        Some(logs.len() - 1)
    } else {
        None
    }
}

/// Find a log at a relative time offset from current position.
/// Negative seconds means go backwards in time.
fn find_by_relative_time(app: &App, logs: &[LogLine], seconds: i64) -> Option<usize> {
    if logs.is_empty() {
        return None;
    }

    // Determine the reference time
    let reference_time = if let Some(selected_id) = app.selected_line_id {
        // Use currently selected log's time
        logs.iter()
            .find(|log| log.id == selected_id)
            .map(|log| log.timestamp)
    } else {
        // Use the last log's time (current tail position)
        logs.last().map(|log| log.timestamp)
    };

    let reference = reference_time?;
    let target = reference + chrono::Duration::seconds(seconds);

    // Find the log closest to target time
    if seconds < 0 {
        // Going backwards: find first log >= target time
        for (idx, log) in logs.iter().enumerate() {
            if log.timestamp >= target {
                return Some(idx);
            }
        }
        // All logs are after target, return first
        Some(0)
    } else {
        // Going forward: find first log >= target time
        for (idx, log) in logs.iter().enumerate() {
            if log.timestamp >= target {
                return Some(idx);
            }
        }
        // No log at or after target, return last
        Some(logs.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;
    use chrono::{Local, NaiveDate, NaiveDateTime};

    fn make_log(_id: u64, hour: u32, minute: u32, second: u32) -> LogLine {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let time = NaiveTime::from_hms_opt(hour, minute, second).unwrap();
        let timestamp = NaiveDateTime::new(date, time).and_local_timezone(Local).unwrap();

        LogLine::new_with_time(
            LogSource::ProcessStdout("test".to_string()),
            format!("Log line at {}:{:02}:{:02}", hour, minute, second),
            timestamp,
        )
    }

    #[test]
    fn test_find_by_absolute_time_exact_match() {
        let logs = vec![
            make_log(1, 10, 0, 0),
            make_log(2, 10, 30, 0),
            make_log(3, 11, 0, 0),
        ];

        let result = find_by_absolute_time(&logs, 10, 30, None);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_find_by_absolute_time_no_exact_match() {
        let logs = vec![
            make_log(1, 10, 0, 0),
            make_log(2, 10, 30, 0),
            make_log(3, 11, 0, 0),
        ];

        // Should find first log at or after 10:15
        let result = find_by_absolute_time(&logs, 10, 15, None);
        assert_eq!(result, Some(1)); // 10:30 is first >= 10:15
    }

    #[test]
    fn test_find_by_absolute_time_before_all() {
        let logs = vec![
            make_log(1, 10, 0, 0),
            make_log(2, 10, 30, 0),
        ];

        let result = find_by_absolute_time(&logs, 9, 0, None);
        assert_eq!(result, Some(0)); // First log
    }

    #[test]
    fn test_find_by_absolute_time_after_all() {
        let logs = vec![
            make_log(1, 10, 0, 0),
            make_log(2, 10, 30, 0),
        ];

        let result = find_by_absolute_time(&logs, 12, 0, None);
        assert_eq!(result, Some(1)); // Last log
    }

    #[test]
    fn test_find_by_absolute_time_with_seconds() {
        let logs = vec![
            make_log(1, 10, 0, 0),
            make_log(2, 10, 0, 30),
            make_log(3, 10, 0, 45),
        ];

        let result = find_by_absolute_time(&logs, 10, 0, Some(30));
        assert_eq!(result, Some(1));
    }
}
