use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::process::ProcessManager;
use super::app::App;
use super::overlays::{draw_help_overlay, draw_expanded_line_overlay, draw_trace_selection_overlay};
use super::widgets::{draw_process_list, draw_log_viewer, draw_status_bar, draw_command_input};

/// Calculate the height needed for the process list based on process count and terminal width
fn calculate_process_list_height(manager: &ProcessManager, app: &App, terminal_width: u16) -> u16 {
    let processes = manager.get_processes();
    if processes.is_empty() {
        return 2; // "No processes" message + border
    }

    // Calculate total width of all process entries
    // Format: "name [Status]" with " | " separators
    let mut total_width: usize = 0;
    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            total_width += 3; // " | " separator
        }

        let handle = &processes[*name];

        // Estimate status text length
        let status_len = if app.hidden_processes.contains(*name) {
            6 // "Hidden"
        } else if let Some((custom_label, _)) = handle.get_custom_status() {
            custom_label.len()
        } else {
            match &handle.status {
                crate::process::ProcessStatus::Running => 7,
                crate::process::ProcessStatus::Stopped => 7,
                crate::process::ProcessStatus::Terminating => 11,
                crate::process::ProcessStatus::Restarting => 10,
                crate::process::ProcessStatus::Failed(_) => 6,
            }
        };

        // "name [Status]" = name.len() + 3 (for " []") + status_len
        total_width += name.len() + 3 + status_len;
    }

    // Calculate how many lines are needed (account for border taking 2 chars width)
    let usable_width = terminal_width.saturating_sub(2) as usize;
    if usable_width == 0 {
        return 2;
    }

    let content_lines = (total_width + usable_width - 1) / usable_width; // Ceiling division
    let content_lines = content_lines.max(1) as u16;

    // Add 1 for the bottom border
    content_lines + 1
}

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &mut App, manager: &ProcessManager) {
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

    // Draw process list
    draw_process_list(f, chunks[0], manager, app);

    // Draw log viewer
    draw_log_viewer(f, chunks[1], manager, app);

    // Draw status bar
    draw_status_bar(f, chunks[2], manager, app);

    // Draw command input
    draw_command_input(f, chunks[3], app);

    // Draw help overlay if show_help is true (must be last so it's on top)
    if app.show_help {
        draw_help_overlay(f, app.help_scroll_offset);
    }

    // Draw expanded line view overlay if enabled (must be last so it's on top)
    if app.expanded_line_view {
        draw_expanded_line_overlay(f, manager, app);
    }

    // Draw trace selection overlay if in trace selection mode
    if app.trace_selection_mode {
        draw_trace_selection_overlay(f, &app.trace_candidates, app.selected_trace_index);
    }
}
