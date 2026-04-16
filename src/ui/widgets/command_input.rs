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
    let muted = app.theme.muted;
    let accent = app.theme.accent;
    let footer_fg = app.theme.footer_fg;
    let prompt = app.theme.success;

    let text = if app.input.search_mode {
        // Show search input with a cursor and help text
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::styled(app.input.input.as_str(), Style::default().fg(footer_fg)),
            Span::styled("_", Style::default().fg(Color::Cyan)),
            Span::styled("  (", Style::default().fg(muted)),
            Span::styled("Enter", Style::default().fg(accent)),
            Span::styled(" to select | ", Style::default().fg(muted)),
            Span::styled("Esc", Style::default().fg(accent)),
            Span::styled(" to cancel)", Style::default().fg(muted)),
        ])
    } else if app.input.command_mode {
        // Show the input with a cursor
        Line::from(vec![
            Span::styled(":", Style::default().fg(prompt)),
            Span::styled(app.input.input.as_str(), Style::default().fg(footer_fg)),
            Span::styled("_", Style::default().fg(prompt)),
        ])
    } else if let Some((message, status_type)) = &app.display.status_message {
        // Show color-coded status message
        let color = match status_type {
            StatusType::Success => app.theme.success,
            StatusType::Error => app.theme.error,
            StatusType::Info => app.theme.info,
        };
        Line::from(vec![Span::styled(message, Style::default().fg(color))])
    } else {
        // Show help text
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(muted)),
            Span::styled(
                ":",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" for commands, ", Style::default().fg(muted)),
            Span::styled(
                "/",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to search, ", Style::default().fg(muted)),
            Span::styled(
                "q",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit", Style::default().fg(muted)),
        ])
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(app.theme.footer_bg).fg(footer_fg));

    f.render_widget(paragraph, area);
}
