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

    // When in batch view mode, we're viewing a single batch and line_idx is relative to it
    let (batch_idx, start, end) = if app.batch_view_mode {
        if let Some(current_batch) = app.current_batch {
            if current_batch < filtered.batches.len() {
                let (s, e) = filtered.batches[current_batch];
                (current_batch, s, e)
            } else {
                return Err("Current batch out of range".to_string());
            }
        } else {
            return Err("No batch selected".to_string());
        }
    } else {
        // Not in batch view mode - find which batch contains the selected line
        filtered.batches.iter().enumerate()
            .find(|(_, (start, end))| line_idx >= *start && line_idx <= *end)
            .map(|(idx, (s, e))| (idx, *s, *e))
            .ok_or_else(|| "No batch found for selected line".to_string())?
    };

    // Format the entire batch
    let mut batch_text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, end - start + 1);

    for log in &filtered.logs[start..=end] {
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
