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

/// Represents a cell to be rendered in the process grid
struct Cell {
    content: Vec<Span<'static>>,
    width: usize,
    process_name: Option<String>,
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
pub fn calculate_row_count(cell_widths: &[usize], max_width: usize) -> usize {
    calculate_row_layout(cell_widths, max_width).len()
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
        let is_noteworthy = is_hidden
            || custom_label.is_some()
            || !matches!(handle.status, ProcessStatus::Running);

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
        ProcessPanelViewMode::Normal => {
            render_grid(&all_cells, None, usable_width, area, app)
        }
        ProcessPanelViewMode::Summary => {
            let noteworthy_cells: Vec<Cell> = noteworthy_indices
                .into_iter()
                .map(|i| {
                    // We need to rebuild cells for noteworthy entries
                    let orig = &all_cells[i];
                    Cell {
                        content: orig.content.clone(),
                        width: orig.width,
                        process_name: orig.process_name.clone(),
                    }
                })
                .collect();

            if noteworthy_cells.is_empty() {
                vec![Line::from(vec![Span::styled(
                    format!("All {} processes running [p to expand]", total_count),
                    Style::default().fg(Color::DarkGray),
                )])]
            } else {
                let suffix = format!("[{} of {}, p to expand]", noteworthy_cells.len(), total_count);
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
    let mut content: Vec<Span<'static>> = Vec::new();

    content.push(Span::styled(
        name.to_string(),
        Style::default().fg(name_color).add_modifier(Modifier::BOLD),
    ));
    content.push(Span::raw(" "));
    content.push(Span::styled("●", Style::default().fg(status_color)));

    let mut width = name.len() + 2; // name + " ●"

    if let Some(label) = custom_label {
        content.push(Span::raw(" "));
        content.push(Span::styled(
            label.to_string(),
            Style::default().fg(status_color),
        ));
        width += 1 + label.len();
    }

    Cell {
        content,
        width,
        process_name: Some(name.to_string()),
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

    // Add separator width (3 chars " │ ") to all but last cell in calculation
    // We add separator to cell width for layout purposes
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

            if is_suffix_cell {
                // This is the suffix cell
                if let Some(s) = suffix {
                    if padding > 0 {
                        spans.push(Span::raw(" ".repeat(padding)));
                    }
                    spans.push(Span::styled(
                        s.to_string(),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            } else {
                // Regular process cell
                let cell = &cells[cell_idx];

                // Add padding before content (for column alignment)
                if padding > 0 {
                    spans.push(Span::raw(" ".repeat(padding)));
                }

                // Record click region
                if let Some(ref name) = cell.process_name {
                    let x_pos = area.x + x_offset as u16 + padding as u16;
                    let y_pos = area.y + row_idx as u16;
                    app.regions.process_regions.push((
                        name.clone(),
                        Rect::new(x_pos, y_pos, cell.width as u16, 1),
                    ));
                }

                // Add cell content
                for span in &cell.content {
                    spans.push(span.clone());
                }

                // Add separator if not last cell in row
                let is_last_in_row = col_idx == row_padding.len() - 1;
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
