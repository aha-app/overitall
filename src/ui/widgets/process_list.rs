use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::process::{ProcessManager, ProcessStatus};
use crate::ui::app::App;

/// Draw the process list at the top of the screen
pub fn draw_process_list(f: &mut Frame, area: Rect, manager: &ProcessManager, app: &mut App) {
    // Clear previous process regions
    app.process_regions.clear();

    let processes = manager.get_processes();

    // Sort process names for consistent display
    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    // Build a horizontal layout of processes with separators
    // Track character position for click region mapping
    let mut spans = Vec::new();
    let mut char_pos: u16 = 0;

    for (i, name) in names.iter().enumerate() {
        let handle = &processes[*name];

        // Add separator between processes (but not before the first one)
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            char_pos += 3; // " | " is 3 chars
        }

        // Check if process is hidden
        // Priority: Hidden > Terminating/Failed > Custom status > Standard status
        let (status_text, color) = if app.hidden_processes.contains(*name) {
            ("Hidden".to_string(), Color::DarkGray)
        } else {
            // Terminating and Failed always override custom status
            match &handle.status {
                ProcessStatus::Terminating => ("Terminating".to_string(), Color::Magenta),
                ProcessStatus::Failed(_) => ("Failed".to_string(), Color::Red),
                _ => {
                    // For other statuses, prefer custom status if configured
                    if let Some((custom_label, custom_color)) = handle.get_custom_status() {
                        let color = custom_color.unwrap_or(Color::Green);
                        (custom_label.to_string(), color)
                    } else {
                        // Fall back to standard status display
                        match &handle.status {
                            ProcessStatus::Running => ("Running".to_string(), Color::Green),
                            ProcessStatus::Stopped => ("Stopped".to_string(), Color::Yellow),
                            ProcessStatus::Restarting => ("Restarting".to_string(), Color::Cyan),
                            // Already handled above, but needed for exhaustive match
                            ProcessStatus::Terminating => ("Terminating".to_string(), Color::Magenta),
                            ProcessStatus::Failed(_) => ("Failed".to_string(), Color::Red),
                        }
                    }
                }
            }
        };

        // Calculate the full width of this process entry: "name [status]"
        let entry_width = name.len() + 3 + status_text.len(); // " [" + status + "]"

        // Record the clickable region for this process
        // Note: area.x is the start of the process list area
        app.process_regions.push((
            (*name).clone(),
            Rect::new(area.x + char_pos, area.y, entry_width as u16, 1),
        ));

        // Add process name and status
        let name_color = app.process_colors.get(name);
        spans.push(Span::styled(
            (*name).clone(),
            Style::default().fg(name_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));

        char_pos += entry_width as u16;
    }

    // Add standalone log files after processes
    let mut log_file_names = manager.get_standalone_log_file_names();
    log_file_names.sort();

    for name in log_file_names.iter() {
        // Add separator if there are any previous items
        if !spans.is_empty() {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            char_pos += 3;
        }

        // Check if log file is hidden
        let (status_text, color) = if app.hidden_processes.contains(name) {
            ("Hidden".to_string(), Color::DarkGray)
        } else {
            ("LOG".to_string(), Color::Cyan)
        };

        // Calculate the full width of this log file entry
        let entry_width = name.len() + 3 + status_text.len();

        // Record the clickable region for this log file
        app.process_regions.push((
            name.clone(),
            Rect::new(area.x + char_pos, area.y, entry_width as u16, 1),
        ));

        // Add log file name and status
        let name_color = app.process_colors.get(name);
        spans.push(Span::styled(
            name.clone(),
            Style::default().fg(name_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));

        char_pos += entry_width as u16;
    }

    // If no processes or log files, show a message
    if spans.is_empty() {
        spans.push(Span::styled(
            "No processes",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Create a single line with all processes
    let line = Line::from(spans);

    // Wrap into a paragraph that can handle text wrapping if needed
    let paragraph = Paragraph::new(vec![line])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(paragraph, area);
}
