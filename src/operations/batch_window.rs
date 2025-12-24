use crate::config::Config;
use crate::process::ProcessManager;
use crate::ui::App;
use super::config::save_config_with_error;
use super::logs::FilteredLogs;

/// Increase batch window by 100ms.
/// Returns the new window value and batch count.
pub fn increase_batch_window(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config
) -> (i64, usize) {
    let new_window = app.batch.batch_window_ms + 100;
    app.batch.set_batch_window(new_window);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, new_window);

    config.batch_window_ms = Some(new_window);
    save_config_with_error(config, app);

    (new_window, filtered.batches.len())
}

/// Decrease batch window by 100ms (minimum 1ms).
/// Returns the new window value and batch count.
pub fn decrease_batch_window(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config
) -> (i64, usize) {
    let new_window = (app.batch.batch_window_ms - 100).max(1);
    app.batch.set_batch_window(new_window);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, new_window);

    config.batch_window_ms = Some(new_window);
    save_config_with_error(config, app);

    (new_window, filtered.batches.len())
}

/// Set batch window to a specific value.
/// Returns the batch count with the new window.
pub fn set_batch_window(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config,
    ms: i64
) -> usize {
    app.batch.set_batch_window(ms);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    let filtered = FilteredLogs::from_manager(manager, &app.filters.filters, ms);

    config.batch_window_ms = Some(ms);
    save_config_with_error(config, app);

    filtered.batches.len()
}
