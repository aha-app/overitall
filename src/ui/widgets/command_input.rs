use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;
use crate::ui::types::StatusType;

/// Draw the command input at the bottom of the screen
pub fn draw_command_input(f: &mut Frame, area: Rect, app: &App) {
    let text = if app.search_mode {
        // Show search input with a cursor and help text
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::raw(&app.input),
            Span::styled("_", Style::default().fg(Color::Cyan)),
            Span::styled("  (", Style::default().fg(Color::Gray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" to select | ", Style::default().fg(Color::Gray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" to cancel)", Style::default().fg(Color::Gray)),
        ])
    } else if app.command_mode {
        // Show the input with a cursor
        Line::from(vec![
            Span::styled(":", Style::default().fg(Color::Green)),
            Span::raw(&app.input),
            Span::styled("_", Style::default().fg(Color::Green)),
        ])
    } else if let Some((message, status_type)) = &app.status_message {
        // Show color-coded status message
        let color = match status_type {
            StatusType::Success => Color::Green,
            StatusType::Error => Color::Red,
            StatusType::Info => Color::Yellow,
        };
        Line::from(vec![Span::styled(message, Style::default().fg(color))])
    } else {
        // Show help text
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::Gray)),
            Span::styled(
                ":",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" for commands, ", Style::default().fg(Color::Gray)),
            Span::styled(
                "/",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to search, ", Style::default().fg(Color::Gray)),
            Span::styled(
                "q",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit", Style::default().fg(Color::Gray)),
        ])
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));

    f.render_widget(paragraph, area);
}
