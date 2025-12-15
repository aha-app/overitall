use crate::process::ProcessManager;
use crate::ui::App;
use super::logs::FilteredLogs;

/// Select the previous line, with wrap-around support.
/// Creates a snapshot on first selection.
/// Returns the new selected index.
pub fn select_prev_line(app: &mut App, manager: &ProcessManager) -> Option<usize> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
    let total_logs = filtered.visible_count(app);

    if total_logs == 0 {
        return None;
    }

    // Create snapshot on first selection
    if app.selected_line_index.is_none() {
        app.create_snapshot(filtered.logs);
    }

    app.select_prev_line(total_logs);
    app.selected_line_index
}

/// Select the next line, with wrap-around support.
/// Creates a snapshot on first selection.
/// Returns the new selected index.
pub fn select_next_line(app: &mut App, manager: &ProcessManager) -> Option<usize> {
    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
    let total_logs = filtered.visible_count(app);

    if total_logs == 0 {
        return None;
    }

    // Create snapshot on first selection
    if app.selected_line_index.is_none() {
        app.create_snapshot(filtered.logs);
    }

    app.select_next_line(total_logs);
    app.selected_line_index
}

/// Move the selection up by a page (20 lines).
/// If no line is selected, scrolls the view instead.
pub fn page_up(app: &mut App, _manager: &ProcessManager) {
    const PAGE_SIZE: usize = 20;

    if app.selected_line_index.is_some() {
        if let Some(current_idx) = app.selected_line_index {
            let new_idx = current_idx.saturating_sub(PAGE_SIZE);
            app.selected_line_index = Some(new_idx);
            app.auto_scroll = false;
        }
    } else {
        app.scroll_up(PAGE_SIZE);
    }
}

/// Move the selection down by a page (20 lines).
/// If no line is selected, scrolls the view instead.
pub fn page_down(app: &mut App, manager: &ProcessManager) {
    const PAGE_SIZE: usize = 20;

    if app.selected_line_index.is_some() {
        let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);
        let total_logs = filtered.visible_count(app);

        if let Some(current_idx) = app.selected_line_index {
            let new_idx = (current_idx + PAGE_SIZE).min(total_logs.saturating_sub(1));
            app.selected_line_index = Some(new_idx);
            app.auto_scroll = false;
        }
    } else {
        let total_logs = manager.get_all_logs().len();
        let max_offset = total_logs.saturating_sub(1);
        app.scroll_down(PAGE_SIZE, max_offset);
    }
}
