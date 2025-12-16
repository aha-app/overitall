use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::{App, detect_batches_from_logs, FilterType};

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
        filtered_logs = filtered_logs
            .into_iter()
            .filter(|log| {
                log.line
                    .to_lowercase()
                    .contains(&active_search_pattern.to_lowercase())
            })
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

/// Find the index of a log line by its ID in the given list.
fn find_index_by_id(logs: &[LogLine], id: u64) -> Option<usize> {
    logs.iter().position(|log| log.id == id)
}

/// Select the previous line, with wrap-around support.
/// Creates a snapshot on first selection.
/// Returns the new selected ID.
pub fn select_prev_line(app: &mut App, manager: &ProcessManager) -> Option<u64> {
    let display_logs = get_display_logs(app, manager);

    if display_logs.is_empty() {
        return None;
    }

    // Create snapshot on first selection (if not already frozen/in trace mode)
    let was_none = app.selected_line_id.is_none();
    if was_none && app.snapshot.is_none() {
        // Get all logs with filters applied for the snapshot
        let logs = manager.get_all_logs();
        let filtered = crate::ui::apply_filters(logs, &app.filters);
        app.create_snapshot(filtered);
    }

    let new_id = match app.selected_line_id {
        None => {
            // When tailing, Up arrow selects the last (most recent) line
            display_logs.last().map(|log| log.id)
        }
        Some(current_id) => {
            // Find current position in display list
            match find_index_by_id(&display_logs, current_id) {
                Some(0) => {
                    // At top, wrap to bottom
                    display_logs.last().map(|log| log.id)
                }
                Some(idx) => {
                    // Move to previous
                    Some(display_logs[idx - 1].id)
                }
                None => {
                    // Current ID not found in display list, select last
                    display_logs.last().map(|log| log.id)
                }
            }
        }
    };

    app.selected_line_id = new_id;
    app.auto_scroll = false;
    if was_none {
        app.freeze_display();
    }
    new_id
}

/// Select the next line, with wrap-around support.
/// Creates a snapshot on first selection.
/// Returns the new selected ID.
pub fn select_next_line(app: &mut App, manager: &ProcessManager) -> Option<u64> {
    let display_logs = get_display_logs(app, manager);

    if display_logs.is_empty() {
        return None;
    }

    // Create snapshot on first selection (if not already frozen/in trace mode)
    let was_none = app.selected_line_id.is_none();
    if was_none && app.snapshot.is_none() {
        // Get all logs with filters applied for the snapshot
        let logs = manager.get_all_logs();
        let filtered = crate::ui::apply_filters(logs, &app.filters);
        app.create_snapshot(filtered);
    }

    let new_id = match app.selected_line_id {
        None => {
            // Start at first line
            display_logs.first().map(|log| log.id)
        }
        Some(current_id) => {
            // Find current position in display list
            let len = display_logs.len();
            match find_index_by_id(&display_logs, current_id) {
                Some(idx) if idx >= len - 1 => {
                    // At bottom, wrap to top
                    display_logs.first().map(|log| log.id)
                }
                Some(idx) => {
                    // Move to next
                    Some(display_logs[idx + 1].id)
                }
                None => {
                    // Current ID not found in display list, select first
                    display_logs.first().map(|log| log.id)
                }
            }
        }
    };

    app.selected_line_id = new_id;
    app.auto_scroll = false;
    if was_none {
        app.freeze_display();
    }
    new_id
}

/// Move the selection up by a page (20 lines).
/// If no line is selected, scrolls the view instead.
pub fn page_up(app: &mut App, manager: &ProcessManager) {
    const PAGE_SIZE: usize = 20;

    if app.selected_line_id.is_some() {
        let display_logs = get_display_logs(app, manager);

        if let Some(current_id) = app.selected_line_id {
            if let Some(current_idx) = find_index_by_id(&display_logs, current_id) {
                let new_idx = current_idx.saturating_sub(PAGE_SIZE);
                app.selected_line_id = Some(display_logs[new_idx].id);
                app.auto_scroll = false;
            }
        }
    } else if app.auto_scroll {
        // When in auto_scroll mode, we're viewing the bottom of the log.
        // Calculate the effective scroll position and move up from there.
        let display_logs = get_display_logs(app, manager);
        let total_logs = display_logs.len();

        // Effective position when auto-scrolling: showing the last PAGE_SIZE*2 lines
        // (we use PAGE_SIZE*2 as a reasonable approximation of visible_lines + PAGE_SIZE)
        // After pressing PageUp, we want to show one page earlier
        let effective_offset = total_logs.saturating_sub(PAGE_SIZE * 2);
        app.scroll_offset = effective_offset;
        app.auto_scroll = false;
    } else {
        app.scroll_up(PAGE_SIZE);
    }
}

/// Move the selection down by a page (20 lines).
/// If no line is selected, scrolls the view instead.
pub fn page_down(app: &mut App, manager: &ProcessManager) {
    const PAGE_SIZE: usize = 20;

    if app.selected_line_id.is_some() {
        let display_logs = get_display_logs(app, manager);
        let total_logs = display_logs.len();

        if let Some(current_id) = app.selected_line_id {
            if let Some(current_idx) = find_index_by_id(&display_logs, current_id) {
                let new_idx = (current_idx + PAGE_SIZE).min(total_logs.saturating_sub(1));
                app.selected_line_id = Some(display_logs[new_idx].id);
                app.auto_scroll = false;
            }
        }
    } else {
        let total_logs = manager.get_all_logs().len();
        let max_offset = total_logs.saturating_sub(1);
        app.scroll_down(PAGE_SIZE, max_offset);
    }
}
