use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::process::ProcessManager;
use super::app::App;
use super::display_state::ProcessPanelViewMode;
use super::overlays::{draw_help_overlay, draw_expanded_line_overlay, draw_expanded_line_panel, draw_trace_selection_overlay};
use super::widgets::{draw_process_list, draw_log_viewer, draw_status_bar, draw_command_input, calculate_grid_params};

/// Width threshold for split-screen view (below this, use overlay)
const SPLIT_VIEW_THRESHOLD: u16 = 160;

/// Calculate the height needed for the process list based on process count and terminal width
fn calculate_process_list_height(manager: &ProcessManager, app: &App, terminal_width: u16) -> u16 {
    // Summary and Minimal modes always use 1 row + 1 border = 2 lines
    match app.display.process_panel_mode {
        ProcessPanelViewMode::Summary | ProcessPanelViewMode::Minimal => return 2,
        ProcessPanelViewMode::Normal => {}
    }

    let processes = manager.get_processes();
    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    let mut log_file_names = manager.get_standalone_log_file_names();
    log_file_names.sort();

    let total_entries = names.len() + log_file_names.len();
    if total_entries == 0 {
        return 2; // "No processes" message + border
    }

    let (column_width, _) = calculate_grid_params(&names, &log_file_names);
    let usable_width = terminal_width.saturating_sub(2) as usize;
    if usable_width == 0 {
        return 2;
    }

    let num_columns = (usable_width / column_width).max(1);
    let num_rows = (total_entries + num_columns - 1) / num_columns; // Ceiling division

    // Add 1 for the bottom border
    (num_rows as u16) + 1
}

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &mut App, manager: &ProcessManager) {
    // Determine if we should use split view mode
    let use_split_view = f.area().width >= SPLIT_VIEW_THRESHOLD && app.display.expanded_line_view;

    // Calculate dynamic height for process list based on number of processes
    let process_list_height = calculate_process_list_height(manager, app, f.area().width);

    // Create the main layout: process list, log viewer, status bar, command input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(process_list_height), // Process list (dynamic height)
            Constraint::Min(0),         // Log viewer (takes remaining space)
            Constraint::Length(1),      // Status bar
            Constraint::Length(1),      // Command input (exactly 1 line)
        ])
        .split(f.area());

    // Store layout areas for mouse click detection
    app.regions.process_list_area = Some(chunks[0]);
    app.regions.status_bar_area = Some(chunks[2]);

    // Draw process list
    draw_process_list(f, chunks[0], manager, app);

    // Draw log area - either split with detail panel or full width
    if use_split_view {
        // Split horizontally: 60% log viewer, 40% detail panel
        let log_area_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(chunks[1]);

        app.regions.log_viewer_area = Some(log_area_chunks[0]);
        draw_log_viewer(f, log_area_chunks[0], manager, app);
        draw_expanded_line_panel(f, log_area_chunks[1], manager, app);
    } else {
        app.regions.log_viewer_area = Some(chunks[1]);
        draw_log_viewer(f, chunks[1], manager, app);
    }

    // Draw status bar
    draw_status_bar(f, chunks[2], manager, app);

    // Draw command input
    draw_command_input(f, chunks[3], app);

    // Draw help overlay if show_help is true (must be last so it's on top)
    if app.display.show_help {
        draw_help_overlay(f, app.display.help_scroll_offset);
    }

    // Draw expanded line view overlay if enabled and NOT in split view mode
    if app.display.expanded_line_view && !use_split_view {
        draw_expanded_line_overlay(f, manager, app);
    }

    // Draw trace selection overlay if in trace selection mode
    if app.trace.trace_selection_mode {
        draw_trace_selection_overlay(f, &app.trace.trace_candidates, app.trace.selected_trace_index);
    }
}
