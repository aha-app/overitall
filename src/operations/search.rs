use crate::process::ProcessManager;
use crate::ui::{App, apply_filters};

/// Execute a search on the filtered logs and set up the selection state.
/// Returns Ok with the match count on success, or Err with an error message.
pub fn execute_search(app: &mut App, manager: &ProcessManager, search_text: &str) -> Result<usize, String> {
    if search_text.is_empty() {
        return Err("Empty search".to_string());
    }

    // Save the search pattern
    app.perform_search(search_text.to_string());

    // Get filtered logs (after persistent filters AND search filter)
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters);

    // Apply search filter
    let search_filtered: Vec<_> = filtered_logs
        .into_iter()
        .filter(|log| {
            log.line
                .to_lowercase()
                .contains(&search_text.to_lowercase())
        })
        .collect();

    if search_filtered.is_empty() {
        return Err("No matches found".to_string());
    }

    let match_count = search_filtered.len();

    // Create snapshot
    app.create_snapshot(search_filtered);

    // Freeze display
    app.freeze_display();

    // Select the last (bottom) entry
    let last_index = match_count.saturating_sub(1);
    app.selected_line_index = Some(last_index);

    // Exit search_mode so user can't type (but keep search_pattern)
    app.search_mode = false;
    app.input.clear();

    Ok(match_count)
}

/// Show the full context around the currently selected log.
/// Clears the search pattern and shows all filtered logs with selection preserved.
/// Returns Ok with success message on success, or Err with an error message.
pub fn show_context(app: &mut App, manager: &ProcessManager) -> Result<String, String> {
    // Close expanded view
    app.close_expanded_view();

    // Get the currently selected log line (before we change anything)
    let selected_log = if let Some(idx) = app.selected_line_index {
        if let Some(snapshot) = &app.snapshot {
            snapshot.get(idx).cloned()
        } else {
            None
        }
    } else {
        None
    };

    let selected_log = match selected_log {
        Some(log) => log,
        None => return Err("No log selected".to_string()),
    };

    // Clear search pattern to show all logs
    app.search_pattern.clear();

    // Get ALL filtered logs (persistent filters only, no search)
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters);

    // Find the index of the selected log in the full filtered set
    // Match by timestamp and line content for uniqueness
    let new_index = filtered_logs.iter().position(|log| {
        log.timestamp == selected_log.timestamp && log.line == selected_log.line
    });

    let new_index = match new_index {
        Some(idx) => idx,
        None => return Err("Could not find log in context".to_string()),
    };

    // Create new snapshot with all logs
    app.create_snapshot(filtered_logs);

    // Update selection to point to the same log in the full context
    app.selected_line_index = Some(new_index);

    // Display is already frozen, keep it that way
    Ok("Showing context around selected log".to_string())
}
