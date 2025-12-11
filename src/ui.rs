use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::process::{ProcessManager, ProcessStatus};

/// Application state for the TUI
pub struct App {
    /// Current command input text
    pub input: String,
    /// Scroll offset for log viewer (lines from bottom)
    pub scroll_offset: usize,
    /// Whether the app should quit
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            scroll_offset: 0,
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &App, manager: &ProcessManager) {
    // Create the main layout: process list, log viewer, command input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10), // Process list
            Constraint::Percentage(85), // Log viewer
            Constraint::Percentage(5),  // Command input
        ])
        .split(f.area());

    // Draw process list
    draw_process_list(f, chunks[0], manager);

    // Draw log viewer
    draw_log_viewer(f, chunks[1], manager, app.scroll_offset);

    // Draw command input
    draw_command_input(f, chunks[2], &app.input);
}

/// Draw the process list at the top of the screen
fn draw_process_list(f: &mut Frame, area: ratatui::layout::Rect, manager: &ProcessManager) {
    let processes = manager.get_all_statuses();

    let items: Vec<ListItem> = processes
        .iter()
        .map(|(name, status)| {
            let (status_text, color) = match status {
                ProcessStatus::Running => ("Running", Color::Green),
                ProcessStatus::Stopped => ("Stopped", Color::Red),
                ProcessStatus::Failed(_) => ("Failed", Color::Red),
            };

            let content = Line::from(vec![
                Span::raw("• "),
                Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" ["),
                Span::styled(status_text, Style::default().fg(color)),
                Span::raw("]"),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Processes ")
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    f.render_widget(list, area);
}

/// Draw the log viewer in the middle of the screen
fn draw_log_viewer(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    manager: &ProcessManager,
    _scroll_offset: usize,
) {
    let logs = manager.get_all_logs();

    // Format log lines: [HH:MM:SS] process_name: message
    let log_lines: Vec<Line> = logs
        .iter()
        .map(|log| {
            let timestamp = log.timestamp.format("%H:%M:%S").to_string();
            let process_name = log.source.process_name();

            Line::from(vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{}: ", process_name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&log.line),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Logs ")
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    f.render_widget(paragraph, area);
}

/// Draw the command input at the bottom of the screen
fn draw_command_input(f: &mut Frame, area: ratatui::layout::Rect, input: &str) {
    let text = if input.is_empty() {
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::Gray)),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::Gray)),
            Span::raw(input),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ])
    };

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}
