use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::process::{ProcessManager, ProcessStatus};
use crate::ui::app::App;
use crate::ui::display_state::ProcessPanelViewMode;

/// Represents an entry in the process list (either a process or a log file)
struct ProcessEntry {
    name: String,
    status_color: Color,
    name_color: Color,
    custom_label: Option<String>,
    is_noteworthy: bool,
}

/// Calculate grid layout parameters for process list
pub fn calculate_grid_params(
    process_names: &[&String],
    log_file_names: &[String],
) -> (usize, usize) {
    let max_process_name = process_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let max_log_name = log_file_names.iter().map(|n| n.len()).max().unwrap_or(0);
    let max_name_len = max_process_name.max(max_log_name);

    // Compact format: "name ●" with " │ " separator between columns
    // " ●" = 2 chars, " │ " = 3 chars
    let column_width = max_name_len + 2 + 3;

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
        let (status_color, custom_label) = get_process_status(handle);
        let is_hidden = app.filters.hidden_processes.contains(*name);
        let name_color = if is_hidden {
            Color::DarkGray
        } else {
            app.process_colors.get(name)
        };
        let is_noteworthy = is_hidden
            || custom_label.is_some()
            || !matches!(handle.status, ProcessStatus::Running);
        entries.push(ProcessEntry {
            name: (*name).clone(),
            status_color,
            name_color,
            custom_label,
            is_noteworthy,
        });
    }

    for name in log_file_names.iter() {
        let is_hidden = app.filters.hidden_processes.contains(name);
        let name_color = if is_hidden {
            Color::DarkGray
        } else {
            app.process_colors.get(name)
        };
        entries.push(ProcessEntry {
            name: name.clone(),
            status_color: Color::Cyan,
            name_color,
            custom_label: None,
            is_noteworthy: is_hidden,
        });
    }

    let total_count = entries.len();
    let view_mode = app.display.process_panel_mode;

    let lines = match view_mode {
        ProcessPanelViewMode::Normal => {
            render_normal_mode(&entries, column_width, num_columns, area, app)
        }
        ProcessPanelViewMode::Summary => {
            let noteworthy: Vec<ProcessEntry> = entries.into_iter().filter(|e| e.is_noteworthy).collect();
            render_summary_mode(&noteworthy, total_count, column_width, num_columns, area, app)
        }
        ProcessPanelViewMode::Minimal => {
            render_minimal_mode(total_count)
        }
    };

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(paragraph, area);
}

/// Render normal mode: full grid layout with all processes
fn render_normal_mode<'a>(
    entries: &[ProcessEntry],
    column_width: usize,
    num_columns: usize,
    area: Rect,
    app: &mut App,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let needs_padding = entries.len() > num_columns;

    for (row_idx, chunk) in entries.chunks(num_columns).enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        for (col_idx, entry) in chunk.iter().enumerate() {
            let max_name_len = column_width.saturating_sub(5);
            let name_padding = if needs_padding {
                max_name_len.saturating_sub(entry.name.len())
            } else {
                0
            };

            let entry_len = entry.name.len()
                + name_padding
                + 2
                + entry.custom_label.as_ref().map(|l| l.len() + 1).unwrap_or(0);

            let x_pos = area.x + (col_idx * column_width) as u16;
            let y_pos = area.y + row_idx as u16;
            app.regions.process_regions.push((
                entry.name.clone(),
                Rect::new(x_pos, y_pos, entry_len as u16, 1),
            ));

            spans.push(Span::styled(
                entry.name.clone(),
                Style::default()
                    .fg(entry.name_color)
                    .add_modifier(Modifier::BOLD),
            ));

            if name_padding > 0 {
                spans.push(Span::raw(" ".repeat(name_padding)));
            }

            spans.push(Span::raw(" "));
            spans.push(Span::styled("●", Style::default().fg(entry.status_color)));

            if let Some(label) = &entry.custom_label {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    label.clone(),
                    Style::default().fg(entry.status_color),
                ));
            }

            if col_idx < chunk.len() - 1 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Render summary mode: process count prefix + grid layout for noteworthy processes only
fn render_summary_mode<'a>(
    noteworthy_entries: &[ProcessEntry],
    total_count: usize,
    column_width: usize,
    num_columns: usize,
    area: Rect,
    app: &mut App,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // First line starts with "Processes: X  " then continues with grid entries
    let prefix = format!("Processes: {}  ", total_count);
    let prefix_len = prefix.len();

    if noteworthy_entries.is_empty() {
        // No noteworthy processes, just show the count
        lines.push(Line::from(vec![Span::styled(
            format!("Processes: {}", total_count),
            Style::default().fg(Color::White),
        )]));
        return lines;
    }

    let needs_padding = noteworthy_entries.len() > num_columns;

    for (row_idx, chunk) in noteworthy_entries.chunks(num_columns).enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        // Add prefix on first row
        if row_idx == 0 {
            spans.push(Span::styled(prefix.clone(), Style::default().fg(Color::White)));
        }

        for (col_idx, entry) in chunk.iter().enumerate() {
            let max_name_len = column_width.saturating_sub(5);
            let name_padding = if needs_padding {
                max_name_len.saturating_sub(entry.name.len())
            } else {
                0
            };

            let entry_len = entry.name.len()
                + name_padding
                + 2
                + entry.custom_label.as_ref().map(|l| l.len() + 1).unwrap_or(0);

            let x_offset = if row_idx == 0 { prefix_len } else { 0 };
            let x_pos = area.x + x_offset as u16 + (col_idx * column_width) as u16;
            let y_pos = area.y + row_idx as u16;
            app.regions.process_regions.push((
                entry.name.clone(),
                Rect::new(x_pos, y_pos, entry_len as u16, 1),
            ));

            spans.push(Span::styled(
                entry.name.clone(),
                Style::default()
                    .fg(entry.name_color)
                    .add_modifier(Modifier::BOLD),
            ));

            if name_padding > 0 {
                spans.push(Span::raw(" ".repeat(name_padding)));
            }

            spans.push(Span::raw(" "));
            spans.push(Span::styled("●", Style::default().fg(entry.status_color)));

            if let Some(label) = &entry.custom_label {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    label.clone(),
                    Style::default().fg(entry.status_color),
                ));
            }

            if col_idx < chunk.len() - 1 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Render minimal mode: just the process count
fn render_minimal_mode(total_count: usize) -> Vec<Line<'static>> {
    vec![Line::from(vec![Span::styled(
        format!("Processes: {}", total_count),
        Style::default().fg(Color::White),
    )])]
}

/// Get status color and optional custom label for a process
fn get_process_status(
    handle: &crate::process::ProcessHandle,
) -> (Color, Option<String>) {
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
