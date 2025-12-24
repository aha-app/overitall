use chrono::Local;
use crate::process::ProcessManager;
use crate::ui::App;

/// Start recording a manual trace
pub fn start_recording(app: &mut App) {
    app.trace.manual_trace_recording = true;
    app.trace.manual_trace_start = Some(Local::now());
    app.set_status_info("Recording trace... press 's' to stop".to_string());
}

/// Stop recording and enter trace filter mode
pub fn stop_recording(app: &mut App, manager: &ProcessManager) -> Result<String, String> {
    let start_time = app.trace.manual_trace_start
        .ok_or_else(|| "No recording in progress".to_string())?;
    let end_time = Local::now();

    // Get all logs in this time window
    let logs = manager.get_all_logs();
    let filtered: Vec<_> = logs.into_iter()
        .filter(|log| log.arrival_time >= start_time && log.arrival_time <= end_time)
        .cloned()
        .collect();

    let log_count = filtered.len();
    let duration = end_time - start_time;
    let duration_ms = duration.num_milliseconds();

    if filtered.is_empty() {
        // Reset state
        app.trace.manual_trace_recording = false;
        app.trace.manual_trace_start = None;
        return Err("No logs captured in time window".to_string());
    }

    // Enter trace filter mode
    app.create_snapshot(filtered);
    app.trace.trace_time_start = Some(start_time);
    app.trace.trace_time_end = Some(end_time);
    app.trace.active_trace_id = None; // No correlation ID for manual traces
    app.trace.trace_filter_mode = true;
    app.freeze_display();

    // Reset recording state
    app.trace.manual_trace_recording = false;
    app.trace.manual_trace_start = None;

    Ok(format!("Captured {} logs in {:.1}s", log_count, duration_ms as f64 / 1000.0))
}

/// Cancel recording without entering trace mode
pub fn cancel_recording(app: &mut App) {
    app.trace.manual_trace_recording = false;
    app.trace.manual_trace_start = None;
}
