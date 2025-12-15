use crate::process::ProcessManager;
use crate::ui::{App, apply_filters};

/// Navigate to the next batch.
/// Creates a snapshot if entering batch view for the first time.
/// Returns true if a snapshot was created (first entry to batch view).
pub fn next_batch(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters);

    let was_none = !app.batch_view_mode;
    if was_none {
        app.create_snapshot(filtered_logs);
    }

    app.next_batch();
    was_none
}

/// Navigate to the previous batch.
/// Creates a snapshot if entering batch view for the first time.
/// Returns true if a snapshot was created (first entry to batch view).
pub fn prev_batch(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters);

    let was_none = !app.batch_view_mode;
    if was_none {
        app.create_snapshot(filtered_logs);
    }

    app.prev_batch();
    was_none
}

/// Toggle batch view mode.
/// Creates a snapshot if entering batch view, discards it if exiting.
/// Returns true if batch view mode is now enabled.
pub fn toggle_batch_view(app: &mut App, manager: &ProcessManager) -> bool {
    let logs = manager.get_all_logs();
    let filtered_logs = apply_filters(logs, &app.filters);

    let entering_batch_view = !app.batch_view_mode;
    if entering_batch_view {
        app.create_snapshot(filtered_logs);
    } else {
        app.discard_snapshot();
    }

    app.toggle_batch_view();
    app.batch_view_mode
}
