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
    let mut processes = manager.get_all_statuses();

    // Sort processes by name for consistent display
    processes.sort_by(|a, b| a.0.cmp(&b.0));

    // Build a horizontal layout of processes with separators
    let mut spans = Vec::new();

    for (i, (name, status)) in processes.iter().enumerate() {
        // Add separator between processes (but not before the first one)
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }

        // Check if process is hidden
        let (status_text, color) = if app.hidden_processes.contains(name) {
            ("Hidden", Color::DarkGray)
        } else {
            match status {
                ProcessStatus::Running => ("Running", Color::Green),
                ProcessStatus::Stopped => ("Stopped", Color::Yellow),
                ProcessStatus::Terminating => ("Terminating", Color::Magenta),
                ProcessStatus::Failed(_) => ("Failed", Color::Red),
            }
        };

        // Add process name and status
        spans.push(Span::styled(
            name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));
    }

    // If no processes, show a message
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
