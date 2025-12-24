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
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    // Apply search filter
    let search_text_lower = search_text.to_lowercase();
    let search_filtered: Vec<_> = filtered_logs
        .into_iter()
        .filter(|log| log.line_lowercase().contains(&search_text_lower))
        .collect();

    if search_filtered.is_empty() {
        return Err("No matches found".to_string());
    }

    let match_count = search_filtered.len();

    // Select the last (bottom) entry by ID
    let last_id = search_filtered.last().map(|log| log.id);
    app.navigation.selected_line_id = last_id;

    // Create snapshot
    app.navigation.create_snapshot(search_filtered);

    // Freeze display
    app.navigation.freeze_display();

    // Exit search_mode so user can't type (but keep search_pattern)
    app.input.search_mode = false;
    app.input.input.clear();

    Ok(match_count)
}

/// Show the full context around the currently selected log.
/// Clears the search pattern and shows all filtered logs with selection preserved.
/// Returns Ok with success message on success, or Err with an error message.
pub fn show_context(app: &mut App, manager: &ProcessManager) -> Result<String, String> {
    // Close expanded view
    app.display.close_expanded_view();

    // Get the currently selected log line by ID (before we change anything)
    let selected_id = app.navigation.selected_line_id
        .ok_or_else(|| "No log selected".to_string())?;

    // Verify the selected ID exists in the snapshot
    if let Some(snapshot) = &app.navigation.snapshot {
        if !snapshot.iter().any(|log| log.id == selected_id) {
            return Err("Selected log not found".to_string());
        }
    } else {
        return Err("No snapshot available".to_string());
    }

    // Clear search pattern to show all logs
    app.input.search_pattern.clear();

    // Get ALL filtered logs (persistent filters only, no search)
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    // Verify the selected ID exists in the full filtered set
    if !filtered_logs.iter().any(|log| log.id == selected_id) {
        return Err("Could not find log in context".to_string());
    }

    // Create new snapshot with all logs
    // The selected_line_id remains the same - it will still point to the same log
    app.navigation.create_snapshot(filtered_logs);

    // Display is already frozen, keep it that way
    Ok("Showing context around selected log".to_string())
}
