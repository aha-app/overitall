use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::process::{ProcessManager, ProcessStatus};
use crate::ui::app::App;
use crate::ui::display_state::ProcessPanelViewMode;

/// Represents a cell to be rendered in the process grid
struct Cell {
    name: String,
    name_color: Color,
    status_color: Color,
    custom_label: Option<String>,
    width: usize,
}

/// Calculate row layout: given cell widths and max width, returns padding for each cell.
/// Returns Vec<Vec<usize>> where each inner Vec is a row containing padding amounts.
/// Finds the optimal number of columns such that all rows fit within max_width.
fn calculate_row_layout(cell_widths: &[usize], max_width: usize) -> Vec<Vec<usize>> {
    if cell_widths.is_empty() {
        return vec![];
    }

    let n = cell_widths.len();

    // Try from max columns down to 1
    for num_cols in (1..=n).rev() {
        // Compute column widths (max width in each column position)
        let mut col_widths = vec![0usize; num_cols];
        for (i, &w) in cell_widths.iter().enumerate() {
            let col = i % num_cols;
            col_widths[col] = col_widths[col].max(w);
        }

        let total: usize = col_widths.iter().sum();
        if total <= max_width {
            // This layout works, compute padding for each cell
            let mut result = Vec::new();
            for chunk in cell_widths.chunks(num_cols) {
                let row_padding: Vec<usize> = chunk
                    .iter()
                    .enumerate()
                    .map(|(col, &w)| col_widths[col] - w)
                    .collect();
                result.push(row_padding);
            }
            return result;
        }
    }

    // Fallback: 1 column, no padding
    cell_widths.iter().map(|_| vec![0]).collect()
}

/// Calculate the number of rows needed for the given cell widths and max width
fn calculate_row_count(cell_widths: &[usize], max_width: usize) -> usize {
    calculate_row_layout(cell_widths, max_width).len()
}

/// Build cell widths for layout calculation based on view mode
fn build_cell_widths(
    manager: &ProcessManager,
    app: &App,
    mode: ProcessPanelViewMode,
) -> (Vec<usize>, usize) {
    let processes = manager.get_processes();
    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    let mut log_file_names = manager.get_standalone_log_file_names();
    log_file_names.sort();

    let total_count = names.len() + log_file_names.len();
    let mut cell_widths: Vec<usize> = Vec::new();

    for name in &names {
        let handle = &processes[*name];
        let is_hidden = app.filters.hidden_processes.contains(*name);
        let has_custom = handle.get_custom_status().is_some();
        let is_noteworthy =
            is_hidden || has_custom || !matches!(handle.status, ProcessStatus::Running);

        let label_len = if let Some((label, _)) = handle.get_custom_status() {
            label.len() + 1
        } else {
            0
        };
        let cell_width = name.len() + 2 + label_len;

        match mode {
            ProcessPanelViewMode::Normal => cell_widths.push(cell_width),
            ProcessPanelViewMode::Summary if is_noteworthy => cell_widths.push(cell_width),
            _ => {}
        }
    }

    for name in &log_file_names {
        let is_hidden = app.filters.hidden_processes.contains(name);
        let cell_width = name.len() + 2;

        match mode {
            ProcessPanelViewMode::Normal => cell_widths.push(cell_width),
            ProcessPanelViewMode::Summary if is_hidden => cell_widths.push(cell_width),
            _ => {}
        }
    }

    // For summary mode, add suffix as a cell
    if mode == ProcessPanelViewMode::Summary && !cell_widths.is_empty() {
        let suffix = format!("[{} of {}, p to expand]", cell_widths.len(), total_count);
        cell_widths.push(suffix.len());
    }

    (cell_widths, total_count)
}

/// Add separator widths to cell widths for layout calculation
fn add_separator_widths(cell_widths: &[usize]) -> Vec<usize> {
    cell_widths
        .iter()
        .enumerate()
        .map(|(i, &w)| if i < cell_widths.len() - 1 { w + 3 } else { w })
        .collect()
}

/// Calculate the height needed for the process list
pub fn calculate_process_list_height(
    manager: &ProcessManager,
    app: &App,
    terminal_width: u16,
) -> u16 {
    let mode = app.display.process_panel_mode;

    // Minimal mode always uses 1 row + 1 border = 2 lines
    if mode == ProcessPanelViewMode::Minimal {
        return 2;
    }

    let (cell_widths, _total_count) = build_cell_widths(manager, app, mode);

    if cell_widths.is_empty() {
        return 2; // Empty or "all running" message + border
    }

    let usable_width = terminal_width.saturating_sub(2) as usize;
    if usable_width == 0 {
        return 2;
    }

    let cell_widths_with_sep = add_separator_widths(&cell_widths);
    let num_rows = calculate_row_count(&cell_widths_with_sep, usable_width);

    (num_rows as u16) + 1 // +1 for border
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

    // Build all cells with their display info
    let mut all_cells: Vec<Cell> = Vec::new();
    let mut noteworthy_indices: Vec<usize> = Vec::new();

    for name in names.iter() {
        let handle = &processes[*name];
        let (status_color, custom_label) = get_process_status(handle);
        let is_hidden = app.filters.hidden_processes.contains(*name);
        let name_color = if is_hidden {
            Color::DarkGray
        } else {
            app.process_colors.get(name)
        };
        let is_noteworthy =
            is_hidden || custom_label.is_some() || !matches!(handle.status, ProcessStatus::Running);

        let cell = build_process_cell(name, name_color, status_color, custom_label.as_deref());
        if is_noteworthy {
            noteworthy_indices.push(all_cells.len());
        }
        all_cells.push(cell);
    }

    for name in log_file_names.iter() {
        let is_hidden = app.filters.hidden_processes.contains(name);
        let name_color = if is_hidden {
            Color::DarkGray
        } else {
            app.process_colors.get(name)
        };

        let cell = build_process_cell(name, name_color, Color::Cyan, None);
        if is_hidden {
            noteworthy_indices.push(all_cells.len());
        }
        all_cells.push(cell);
    }

    let total_count = all_cells.len();
    let usable_width = area.width.saturating_sub(2) as usize;
    let view_mode = app.display.process_panel_mode;

    let lines = match view_mode {
        ProcessPanelViewMode::Normal => render_grid(&all_cells, None, usable_width, area, app),
        ProcessPanelViewMode::Summary => {
            let noteworthy_cells: Vec<Cell> = noteworthy_indices
                .into_iter()
                .map(|i| {
                    let orig = &all_cells[i];
                    Cell {
                        name: orig.name.clone(),
                        name_color: orig.name_color,
                        status_color: orig.status_color,
                        custom_label: orig.custom_label.clone(),
                        width: orig.width,
                    }
                })
                .collect();

            if noteworthy_cells.is_empty() {
                vec![Line::from(vec![Span::styled(
                    format!("All {} processes running [p to expand]", total_count),
                    Style::default().fg(Color::DarkGray),
                )])]
            } else {
                let suffix = format!(
                    "[{} of {}, p to expand]",
                    noteworthy_cells.len(),
                    total_count
                );
                render_grid(&noteworthy_cells, Some(&suffix), usable_width, area, app)
            }
        }
        ProcessPanelViewMode::Minimal => {
            vec![Line::from(vec![Span::styled(
                format!("{} processes [p to expand]", total_count),
                Style::default().fg(Color::DarkGray),
            )])]
        }
    };

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(paragraph, area);
}

/// Build a cell for a process/log entry
fn build_process_cell(
    name: &str,
    name_color: Color,
    status_color: Color,
    custom_label: Option<&str>,
) -> Cell {
    let label = custom_label.map(|s| s.to_string());
    let width = name.len() + 2 + label.as_ref().map(|l| l.len() + 1).unwrap_or(0);

    Cell {
        name: name.to_string(),
        name_color,
        status_color,
        custom_label: label,
        width,
    }
}

/// Render a grid of cells with optional suffix
fn render_grid<'a>(
    cells: &[Cell],
    suffix: Option<&str>,
    max_width: usize,
    area: Rect,
    app: &mut App,
) -> Vec<Line<'a>> {
    if cells.is_empty() {
        return vec![];
    }

    // Build cell widths array, including suffix as a cell if present
    let mut cell_widths: Vec<usize> = cells.iter().map(|c| c.width).collect();
    if let Some(s) = suffix {
        cell_widths.push(s.len());
    }

    // Add separator width (3 chars " │ ") to all but last cell
    let cell_widths_with_sep: Vec<usize> = cell_widths
        .iter()
        .enumerate()
        .map(|(i, &w)| if i < cell_widths.len() - 1 { w + 3 } else { w })
        .collect();

    let layout = calculate_row_layout(&cell_widths_with_sep, max_width);

    let mut lines: Vec<Line> = Vec::new();
    let mut cell_idx = 0;
    let total_cells = cells.len();

    for (row_idx, row_padding) in layout.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        let mut x_offset = 0usize;

        for (col_idx, &padding) in row_padding.iter().enumerate() {
            let is_suffix_cell = cell_idx >= total_cells;
            let is_last_in_row = col_idx == row_padding.len() - 1;

            if is_suffix_cell {
                // Suffix cell
                if let Some(s) = suffix {
                    if padding > 0 {
                        spans.push(Span::raw(" ".repeat(padding)));
                    }
                    spans.push(Span::styled(s.to_string(), Style::default().fg(Color::DarkGray)));
                }
            } else {
                // Regular process cell - build spans inline
                let cell = &cells[cell_idx];

                // Record click region
                let x_pos = area.x + x_offset as u16;
                let y_pos = area.y + row_idx as u16;
                app.regions.process_regions.push((
                    cell.name.clone(),
                    Rect::new(x_pos, y_pos, (cell.width + padding) as u16, 1),
                ));

                // Name
                spans.push(Span::styled(
                    cell.name.clone(),
                    Style::default().fg(cell.name_color).add_modifier(Modifier::BOLD),
                ));

                // Status dot
                spans.push(Span::raw(" "));
                spans.push(Span::styled("●", Style::default().fg(cell.status_color)));

                // Custom label if present
                if let Some(ref label) = cell.custom_label {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(label.clone(), Style::default().fg(cell.status_color)));
                }

                // Padding at end (for column alignment)
                if padding > 0 {
                    spans.push(Span::raw(" ".repeat(padding)));
                }

                // Separator if not last cell in row
                if !is_last_in_row {
                    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }

                x_offset += padding + cell.width + if is_last_in_row { 0 } else { 3 };
            }

            cell_idx += 1;
        }

        lines.push(Line::from(spans));
    }

    lines
}

/// Get status color and optional custom label for a process
fn get_process_status(handle: &crate::process::ProcessHandle) -> (Color, Option<String>) {
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
