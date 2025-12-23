use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::{self, App, Filter};

/// Holds filtered logs and their detected batches.
/// This struct consolidates the common pattern of:
/// 1. Getting all logs from the process manager
/// 2. Applying filters
/// 3. Detecting batches
pub struct FilteredLogs {
    pub logs: Vec<LogLine>,
    pub batches: Vec<(usize, usize)>,
}

impl FilteredLogs {
    /// Create a new FilteredLogs from a ProcessManager, applying filters and detecting batches.
    pub fn from_manager(manager: &ProcessManager, filters: &[Filter], batch_window_ms: i64) -> Self {
        let logs = manager.get_all_logs();
        let filtered = ui::apply_filters(logs, filters);
        let refs: Vec<&LogLine> = filtered.iter().collect();
        let batches = ui::detect_batches_from_logs(&refs, batch_window_ms);
        Self { logs: filtered, batches }
    }

    /// Calculate the visible count based on batch view mode.
    /// Returns the number of logs visible in the current view.
    #[allow(dead_code)]
    pub fn visible_count(&self, app: &App) -> usize {
        if app.batch_view_mode {
            if let Some(batch_idx) = app.current_batch {
                if !self.batches.is_empty() && batch_idx < self.batches.len() {
                    let (start, end) = self.batches[batch_idx];
                    return end - start + 1;
                }
            }
        }
        self.logs.len()
    }
}
