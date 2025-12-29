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

/// Calculate grid params from ProcessEntry slice
fn calculate_grid_params_for_entries(entries: &[ProcessEntry], usable_width: usize) -> (usize, usize) {
    if entries.is_empty() {
        return (1, 1);
    }
    // Calculate max entry width including name + " ●" + optional " label"
    let max_entry_width = entries
        .iter()
        .map(|e| {
            let label_len = e.custom_label.as_ref().map(|l| l.len() + 1).unwrap_or(0);
            e.name.len() + 2 + label_len // name + " ●" + optional " label"
        })
        .max()
        .unwrap_or(0);
    // Add separator width " │ " = 3 chars
    let column_width = max_entry_width + 3;
    let num_columns = if usable_width > 0 && column_width > 0 {
        (usable_width / column_width).max(1)
    } else {
        1
    };
    (column_width, num_columns)
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
            // Recalculate grid params for noteworthy entries only
            let (nw_column_width, nw_num_columns) = calculate_grid_params_for_entries(&noteworthy, usable_width);
            render_summary_mode(&noteworthy, total_count, nw_column_width, nw_num_columns, area, app)
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

/// Render summary mode: grid layout for noteworthy processes with count suffix
fn render_summary_mode<'a>(
    noteworthy_entries: &[ProcessEntry],
    total_count: usize,
    column_width: usize,
    num_columns: usize,
    area: Rect,
    app: &mut App,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let noteworthy_count = noteworthy_entries.len();
    let suffix = format!("[{} of {}, p to expand]", noteworthy_count, total_count);
    let suffix_width = suffix.len() + 2; // +2 for spacing

    if noteworthy_entries.is_empty() {
        // No noteworthy processes - show message
        lines.push(Line::from(vec![Span::styled(
            format!("All {} processes running [p to expand]", total_count),
            Style::default().fg(Color::DarkGray),
        )]));
        return lines;
    }

    let usable_width = area.width.saturating_sub(2) as usize;
    let needs_padding = noteworthy_entries.len() > num_columns;

    // Calculate how many entries fit on last row with suffix
    let last_row_max_columns = if column_width > 0 {
        ((usable_width.saturating_sub(suffix_width)) / column_width).max(1)
    } else {
        1
    };

    // Build rows manually to handle last row specially
    let mut remaining: &[ProcessEntry] = noteworthy_entries;
    let mut row_idx = 0;

    while !remaining.is_empty() {
        let is_last_row = remaining.len() <= last_row_max_columns;
        let cols_this_row = if is_last_row {
            remaining.len().min(last_row_max_columns)
        } else {
            remaining.len().min(num_columns)
        };

        let (chunk, rest) = remaining.split_at(cols_this_row);
        remaining = rest;

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

        // Add suffix on last row
        if remaining.is_empty() {
            spans.push(Span::styled(
                format!("  {}", suffix),
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(spans));
        row_idx += 1;
    }

    lines
}

/// Render minimal mode: just process count info
fn render_minimal_mode(total_count: usize) -> Vec<Line<'static>> {
    vec![Line::from(vec![Span::styled(
        format!("{} processes [p to expand]", total_count),
        Style::default().fg(Color::DarkGray),
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
