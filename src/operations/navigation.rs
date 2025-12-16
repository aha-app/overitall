use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::App;
use super::logs::FilteredLogs;

/// Get the list of logs to display based on current view mode.
/// Returns owned LogLines for consistent filtering.
fn get_display_logs(app: &App, filtered: &FilteredLogs) -> Vec<LogLine> {
    if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if !filtered.batches.is_empty() && batch_idx < filtered.batches.len() {
                let (start, end) = filtered.batches[batch_idx];
                return filtered.logs[start..=end].to_vec();
            }
        }
    }
    filtered.logs.clone()
}

/// Find the index of a log line by its ID in the given list.
fn find_index_by_id(logs: &[LogLine], id: u64) -> Option<usize> {
    logs.iter().position(|log| log.id == id)
}

/// Select the previous line, with wrap-around support.
/// Creates a snapshot on first selection.
/// Returns the new selected ID.
pub fn select_prev_line(app: &mut App, manager: &ProcessManager) -> Option<u64> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
    let display_logs = get_display_logs(app, &filtered);

    if display_logs.is_empty() {
        return None;
    }

    // Create snapshot on first selection
    let was_none = app.selected_line_id.is_none();
    if was_none {
        app.create_snapshot(filtered.logs.clone());
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
    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
    let display_logs = get_display_logs(app, &filtered);

    if display_logs.is_empty() {
        return None;
    }

    // Create snapshot on first selection
    let was_none = app.selected_line_id.is_none();
    if was_none {
        app.create_snapshot(filtered.logs.clone());
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
        let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
        let display_logs = get_display_logs(app, &filtered);

        if let Some(current_id) = app.selected_line_id {
            if let Some(current_idx) = find_index_by_id(&display_logs, current_id) {
                let new_idx = current_idx.saturating_sub(PAGE_SIZE);
                app.selected_line_id = Some(display_logs[new_idx].id);
                app.auto_scroll = false;
            }
        }
    } else {
        app.scroll_up(PAGE_SIZE);
    }
}

/// Move the selection down by a page (20 lines).
/// If no line is selected, scrolls the view instead.
pub fn page_down(app: &mut App, manager: &ProcessManager) {
    const PAGE_SIZE: usize = 20;

    if app.selected_line_id.is_some() {
        let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
        let display_logs = get_display_logs(app, &filtered);
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
