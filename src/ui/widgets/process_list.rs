use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::process::{ProcessManager, ProcessStatus};
use crate::ui::app::App;

/// Represents an entry in the process list (either a process or a log file)
struct ProcessEntry {
    name: String,
    status_color: Color,
    name_color: Color,
    custom_label: Option<String>,
}

/// Calculate grid layout parameters for process list
pub fn calculate_grid_params(
    process_names: &[&String],
    log_file_names: &[String],
) -> (usize, usize) {
    let max_process_name = process_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let max_log_name = log_file_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let max_name_len = max_process_name.max(max_log_name);

    // Compact format: "name ●" with 2 chars spacing between columns
    // " ●" = 2 chars (space + dot)
    let column_width = max_name_len + 2 + 2;

    let total_entries = process_names.len() + log_file_names.len();

    (column_width, total_entries)
}

/// Draw the process list at the top of the screen
pub fn draw_process_list(f: &mut Frame, area: Rect, manager: &ProcessManager, app: &mut App) {
    app.regions.process_regions.clear();

    let processes = manager.get_processes();

    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    let mut log_file_names = manager.get_standalone_log_file_names();
    log_file_names.sort();

    // Handle empty case
    if names.is_empty() && log_file_names.is_empty() {
        let line = Line::from(vec![Span::styled(
            "No processes",
            Style::default().fg(Color::DarkGray),
        )]);
        let paragraph = Paragraph::new(vec![line]).block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(paragraph, area);
        return;
    }

    // Calculate grid dimensions
    let (column_width, _) = calculate_grid_params(&names, &log_file_names);
    let usable_width = area.width.saturating_sub(2) as usize; // Account for borders
    let num_columns = (usable_width / column_width).max(1);

    // Build entries with their display info
    let mut entries: Vec<ProcessEntry> = Vec::new();

    for name in names.iter() {
        let handle = &processes[*name];
        let (status_color, custom_label) = get_process_status(name, handle, app);
        let name_color = app.process_colors.get(name);
        entries.push(ProcessEntry {
            name: (*name).clone(),
            status_color,
            name_color,
            custom_label,
        });
    }

    for name in log_file_names.iter() {
        let status_color = if app.filters.hidden_processes.contains(name) {
            Color::DarkGray
        } else {
            Color::Cyan
        };
        let name_color = app.process_colors.get(name);
        entries.push(ProcessEntry {
            name: name.clone(),
            status_color,
            name_color,
            custom_label: None,
        });
    }

    // Build lines for grid layout
    let mut lines: Vec<Line> = Vec::new();

    for (row_idx, chunk) in entries.chunks(num_columns).enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        for (col_idx, entry) in chunk.iter().enumerate() {
            // Calculate entry length: "name ●" or "name ● label" for custom status
            let entry_len = entry.name.len()
                + 2
                + entry.custom_label.as_ref().map(|l| l.len() + 1).unwrap_or(0);

            // Record clickable region
            let x_pos = area.x + (col_idx * column_width) as u16;
            let y_pos = area.y + row_idx as u16;
            app.regions.process_regions.push((
                entry.name.clone(),
                Rect::new(x_pos, y_pos, entry_len as u16, 1),
            ));

            // Add name span
            spans.push(Span::styled(
                entry.name.clone(),
                Style::default()
                    .fg(entry.name_color)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled("●", Style::default().fg(entry.status_color)));

            // Add custom label if present
            if let Some(label) = &entry.custom_label {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    label.clone(),
                    Style::default().fg(entry.status_color),
                ));
            }

            // Add padding to align columns (except for last column in row)
            if col_idx < num_columns - 1 {
                let padding_needed = column_width.saturating_sub(entry_len);
                if padding_needed > 0 {
                    spans.push(Span::raw(" ".repeat(padding_needed)));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(paragraph, area);
}

/// Get status color and optional custom label for a process
fn get_process_status(
    name: &str,
    handle: &crate::process::ProcessHandle,
    app: &App,
) -> (Color, Option<String>) {
    if app.filters.hidden_processes.contains(name) {
        return (Color::DarkGray, None);
    }

    match &handle.status {
        ProcessStatus::Terminating => (Color::Magenta, None),
        ProcessStatus::Failed(_) => (Color::Red, None),
        _ => {
            if let Some((custom_label, custom_color)) = handle.get_custom_status() {
                let color = custom_color.unwrap_or(Color::Green);
                (color, Some(custom_label.to_string()))
            } else {
                match &handle.status {
                    ProcessStatus::Running => (Color::Green, None),
                    ProcessStatus::Stopped => (Color::Yellow, None),
                    ProcessStatus::Restarting => (Color::Cyan, None),
                    ProcessStatus::Terminating => (Color::Magenta, None),
                    ProcessStatus::Failed(_) => (Color::Red, None),
                }
            }
        }
    }
}
