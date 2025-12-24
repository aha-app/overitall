use crate::process::ProcessManager;
use crate::traces::detect_traces;
use crate::ui::App;

/// Execute the :traces command - detect traces and enter trace selection mode
pub fn execute_traces(app: &mut App, manager: &ProcessManager) {
    // Get all logs (already returns Vec<&LogLine>)
    let logs = manager.get_all_logs();

    if logs.is_empty() {
        app.set_status_info("No logs available to scan for traces".to_string());
        return;
    }

    // detect_traces expects &[&LogLine], and logs is Vec<&LogLine>
    let candidates = detect_traces(&logs);

    if candidates.is_empty() {
        app.set_status_info("No trace IDs detected in logs".to_string());
        return;
    }

    let count = candidates.len();
    app.enter_trace_selection(candidates);
    app.set_status_info(format!("Found {} trace(s) - select one to filter", count));
}

/// Apply the selected trace filter and enter trace filter mode
pub fn select_trace(app: &mut App, manager: &ProcessManager) {
    if let Some(candidate) = app.get_selected_trace().cloned() {
        // Exit trace selection mode
        app.exit_trace_selection();

        // Create snapshot of logs before entering trace filter mode (clone since we need owned)
        let logs: Vec<_> = manager.get_all_logs().into_iter().cloned().collect();
        app.create_snapshot(logs);

        // Enter trace filter mode
        app.enter_trace_filter(
            candidate.token.clone(),
            candidate.first_occurrence,
            candidate.last_occurrence,
        );

        app.set_status_info(format!(
            "Trace: {} ({} lines) - [ ] to expand, Esc to exit",
            &candidate.token[..8.min(candidate.token.len())],
            candidate.line_count
        ));
    }
}

/// Expand trace view backward
pub fn expand_trace_before(app: &mut App) {
    app.expand_trace_before();
    let secs = app.trace.trace_expand_before.num_seconds();
    app.set_status_info(format!("Expanded trace view: -{}s before", secs));
}

/// Expand trace view forward
pub fn expand_trace_after(app: &mut App) {
    app.expand_trace_after();
    let secs = app.trace.trace_expand_after.num_seconds();
    app.set_status_info(format!("Expanded trace view: +{}s after", secs));
}
