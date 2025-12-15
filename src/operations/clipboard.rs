use anyhow::Result as AnyhowResult;
use arboard::Clipboard;

use crate::operations::logs::FilteredLogs;
use crate::process::ProcessManager;
use crate::ui::App;

fn copy_to_clipboard(text: &str) -> AnyhowResult<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Copy the selected line to clipboard.
/// Returns Ok with success message or Err with error message.
pub fn copy_line(app: &App, manager: &ProcessManager) -> Result<String, String> {
    let line_idx = app.selected_line_index
        .ok_or_else(|| "No line selected".to_string())?;

    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs = if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if !filtered.batches.is_empty() && batch_idx < filtered.batches.len() {
                let (start, end) = filtered.batches[batch_idx];
                filtered.logs[start..=end].to_vec()
            } else {
                filtered.logs
            }
        } else {
            filtered.logs
        }
    } else {
        filtered.logs
    };

    if line_idx >= display_logs.len() {
        return Err("Selected line out of range".to_string());
    }

    let log = &display_logs[line_idx];
    let formatted = format!(
        "[{}] {}: {}",
        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
        log.source.process_name(),
        log.line
    );

    copy_to_clipboard(&formatted)
        .map(|_| "Copied line to clipboard".to_string())
        .map_err(|e| format!("Failed to copy: {}", e))
}

/// Copy the batch containing the selected line to clipboard.
/// Returns Ok with success message or Err with error message.
pub fn copy_batch(app: &App, manager: &ProcessManager) -> Result<String, String> {
    let line_idx = app.selected_line_index
        .ok_or_else(|| "No line selected".to_string())?;

    let filtered = FilteredLogs::from_manager(manager, &app.filters, app.batch_window_ms);

    // Find which batch contains the selected line
    let (batch_idx, (start, end)) = filtered.batches.iter().enumerate()
        .find(|(_, (start, end))| line_idx >= *start && line_idx <= *end)
        .ok_or_else(|| "No batch found for selected line".to_string())?;

    // Format the entire batch
    let mut batch_text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, end - start + 1);

    for log in &filtered.logs[*start..=*end] {
        batch_text.push_str(&format!(
            "[{}] {}: {}\n",
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.source.process_name(),
            log.line
        ));
    }

    let line_count = end - start + 1;
    copy_to_clipboard(&batch_text)
        .map(|_| format!("Copied batch to clipboard ({} lines)", line_count))
        .map_err(|e| format!("Failed to copy: {}", e))
}
