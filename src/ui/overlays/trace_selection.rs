use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::traces::TraceCandidate;
use crate::ui::utils::centered_rect;

/// Draw the trace selection overlay
pub fn draw_trace_selection_overlay(
    f: &mut Frame,
    candidates: &[TraceCandidate],
    selected_index: usize,
) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("Traces ({} found)", candidates.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    // Show up to 10 candidates at a time, scrolling if needed
    let visible_count = 10.min(candidates.len());
    let scroll_offset = if selected_index >= visible_count {
        selected_index - visible_count + 1
    } else {
        0
    };

    // Calculate available width for context preview (account for borders and prefix)
    let available_width = (f.area().width as usize).saturating_sub(6);

    for (idx, candidate) in candidates.iter().enumerate().skip(scroll_offset).take(visible_count) {
        let is_selected = idx == selected_index;
        let prefix = if is_selected { "> " } else { "  " };

        // Format the time (HH:MM:SS)
        let time_str = candidate.first_occurrence.format("%H:%M:%S").to_string();

        // Show token (truncated if needed)
        let token_display = if candidate.token.len() > 36 {
            format!("{}...", &candidate.token[..36])
        } else {
            candidate.token.clone()
        };

        // First line: time, count, and token
        let header = format!(
            "{}{} | {:>3} lines | {}",
            prefix, time_str, candidate.line_count, token_display
        );

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(header, style)));

        // Second line: context preview (indented)
        let preview_indent = "       "; // Align with content after prefix
        let max_preview_len = available_width.saturating_sub(preview_indent.len());
        let preview = if candidate.context_preview.len() > max_preview_len {
            format!("{}...", &candidate.context_preview[..max_preview_len.saturating_sub(3)])
        } else {
            candidate.context_preview.clone()
        };

        let preview_style = if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(Span::styled(
            format!("{}{}", preview_indent, preview),
            preview_style,
        )));

        // Add blank line between entries for readability
        if idx < scroll_offset + visible_count - 1 {
            lines.push(Line::from(""));
        }
    }

    // Show scroll indicator if there are more items
    if candidates.len() > visible_count {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  ... showing {}-{} of {}",
                scroll_offset + 1,
                (scroll_offset + visible_count).min(candidates.len()),
                candidates.len()
            ),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate, "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" select, "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]));

    let block = Block::default()
        .title(" Select Trace ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });

    let area = centered_rect(95, 60, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
