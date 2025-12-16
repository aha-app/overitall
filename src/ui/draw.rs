use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::process::ProcessManager;
use super::app::App;
use super::overlays::{draw_help_overlay, draw_expanded_line_overlay, draw_trace_selection_overlay};
use super::widgets::{draw_process_list, draw_log_viewer, draw_status_bar, draw_command_input};

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &App, manager: &ProcessManager) {
    // Create the main layout: process list, log viewer, status bar, command input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Process list (2 content + 1 separator)
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
        draw_help_overlay(f);
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
