use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::{self, App, apply_filters};

/// Navigate to the next batch.
/// Creates a snapshot if entering batch view for the first time.
/// Returns true if a snapshot was created (first entry to batch view).
pub fn next_batch(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    let was_none = !app.batch.batch_view_mode;
    if was_none {
        app.navigation.create_snapshot(filtered_logs);
    }

    app.next_batch();
    was_none
}

/// Navigate to the previous batch.
/// Creates a snapshot if entering batch view for the first time.
/// Returns true if a snapshot was created (first entry to batch view).
pub fn prev_batch(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    let was_none = !app.batch.batch_view_mode;
    if was_none {
        app.navigation.create_snapshot(filtered_logs);
    }

    app.prev_batch();
    was_none
}

/// Toggle batch view mode.
/// Creates a snapshot if entering batch view, discards it if exiting.
/// Returns true if batch view mode is now enabled.
pub fn toggle_batch_view(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    let entering_batch_view = !app.batch.batch_view_mode;
    if entering_batch_view {
        app.navigation.create_snapshot(filtered_logs);
    } else {
        app.navigation.discard_snapshot();
    }

    app.toggle_batch_view();
    app.batch.batch_view_mode
}

/// Focus on the batch containing the currently selected line.
/// Enters batch view mode and navigates to the batch.
/// Returns Ok with status message on success, Err with error message on failure.
pub fn focus_batch(app: &mut App, manager: &ProcessManager) -> Result<String, String> {
    let selected_id = match app.navigation.selected_line_id {
        Some(id) => id,
        None => return Err("No line selected".to_string()),
    };

    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters.filters);

    let line_idx = match filtered_logs.iter().position(|log| log.id == selected_id) {
        Some(idx) => idx,
        None => return Err("Selected line not found".to_string()),
    };

    let filtered_refs: Vec<&LogLine> = filtered_logs.iter().collect();
    let batches = ui::detect_batches_from_logs(&filtered_refs, app.batch.batch_window_ms);

    let batch_idx = batches
        .iter()
        .enumerate()
        .find(|(_, (start, end))| line_idx >= *start && line_idx <= *end)
        .map(|(idx, _)| idx);

    match batch_idx {
        Some(idx) => {
            if !app.batch.batch_view_mode {
                app.navigation.create_snapshot(filtered_logs);
            }
            app.batch.current_batch = Some(idx);
            app.batch.batch_view_mode = true;
            app.navigation.scroll_offset = 0;
            Ok(format!("Focused on batch {}", idx + 1))
        }
        None => Err("No batch found for selected line".to_string()),
    }
}
