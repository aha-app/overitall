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
pub fn draw_process_list(f: &mut Frame, area: Rect, manager: &ProcessManager, app: &App) {
    let processes = manager.get_processes();

    // Sort process names for consistent display
    let mut names: Vec<&String> = processes.keys().collect();
    names.sort();

    // Build a horizontal layout of processes with separators
    let mut spans = Vec::new();

    for (i, name) in names.iter().enumerate() {
        let handle = &processes[*name];

        // Add separator between processes (but not before the first one)
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }

        // Check if process is hidden
        let (status_text, color) = if app.hidden_processes.contains(*name) {
            ("Hidden".to_string(), Color::DarkGray)
        } else if let Some((custom_label, custom_color)) = handle.get_custom_status() {
            // Use custom status label and color if configured
            let color = custom_color.unwrap_or(Color::Green);
            (custom_label.to_string(), color)
        } else {
            // Fall back to standard status display
            match &handle.status {
                ProcessStatus::Running => ("Running".to_string(), Color::Green),
                ProcessStatus::Stopped => ("Stopped".to_string(), Color::Yellow),
                ProcessStatus::Terminating => ("Terminating".to_string(), Color::Magenta),
                ProcessStatus::Restarting => ("Restarting".to_string(), Color::Cyan),
                ProcessStatus::Failed(_) => ("Failed".to_string(), Color::Red),
            }
        };

        // Add process name and status
        spans.push(Span::styled(
            (*name).clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));
    }

    // Add standalone log files after processes
    let mut log_file_names = manager.get_standalone_log_file_names();
    log_file_names.sort();

    for name in log_file_names.iter() {
        // Add separator if there are any previous items
        if !spans.is_empty() {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }

        // Check if log file is hidden
        let (status_text, color) = if app.hidden_processes.contains(name) {
            ("Hidden".to_string(), Color::DarkGray)
        } else {
            ("LOG".to_string(), Color::Cyan)
        };

        // Add log file name and status
        spans.push(Span::styled(
            name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));
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
