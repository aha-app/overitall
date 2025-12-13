use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::process::ProcessManager;
use crate::ui::app::App;
use crate::ui::batch::detect_batches_from_logs;
use crate::ui::filter::FilterType;
use crate::ui::utils::centered_rect;

/// Draw the expanded line view overlay
pub fn draw_expanded_line_overlay(f: &mut Frame, manager: &ProcessManager, app: &App) {
    // Get the selected line if available
    let selected_line_index = match app.selected_line_index {
        Some(idx) => idx,
        None => {
            // No line selected, don't show the overlay
            return;
        }
    };

    // Get all logs and apply filters (same logic as in draw_log_viewer)
    let logs = manager.get_all_logs();

    let filtered_logs: Vec<&crate::log::LogLine> = if app.filters.is_empty() {
        logs
    } else {
        logs.into_iter()
            .filter(|log| {
                let line_text = &log.line;

                // Check exclude filters first
                for filter in &app.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches(line_text) {
                            return false;
                        }
                    }
                }

                // Check include filters
                let include_filters: Vec<_> = app
                    .filters
                    .iter()
                    .filter(|f| matches!(f.filter_type, FilterType::Include))
                    .collect();

                if include_filters.is_empty() {
                    return true;
                }

                include_filters.iter().any(|filter| filter.matches(line_text))
            })
            .collect()
    };

    // Detect batches
    let batches = detect_batches_from_logs(&filtered_logs, app.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs: Vec<&crate::log::LogLine> = if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if !batches.is_empty() && batch_idx < batches.len() {
                let (start, end) = batches[batch_idx];
                filtered_logs[start..=end].to_vec()
            } else {
                filtered_logs
            }
        } else {
            filtered_logs
        }
    } else {
        filtered_logs
    };

    // Check if selected line index is valid
    if selected_line_index >= display_logs.len() {
        return;
    }

    let selected_log = display_logs[selected_line_index];

    // Find which batch this line belongs to
    let batch_info = if !batches.is_empty() {
        batches.iter().enumerate().find(|(_, (start, end))| {
            selected_line_index >= *start && selected_line_index <= *end
        }).map(|(batch_idx, _)| batch_idx + 1)
    } else {
        None
    };

    // Build the overlay content
    let mut content = vec![
        Line::from(vec![
            Span::styled("Expanded Log Line", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    // Add metadata
    content.push(Line::from(vec![
        Span::styled("Timestamp: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            selected_log.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    content.push(Line::from(vec![
        Span::styled("Process: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            selected_log.source.process_name(),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if let Some(batch_num) = batch_info {
        content.push(Line::from(vec![
            Span::styled("Batch: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}", batch_num),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    content.push(Line::from(vec![
        Span::styled("Line: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{} of {}", selected_line_index + 1, display_logs.len()),
            Style::default().fg(Color::Magenta),
        ),
    ]));

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled("Message:", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    content.push(Line::from(""));

    // Add the full message content (word-wrapped by Paragraph widget)
    content.push(Line::from(selected_log.line.clone()));

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled("Press ", Style::default()),
        Span::styled("ESC", Style::default().fg(Color::Yellow)),
        Span::styled(" to close | ", Style::default()),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" to show context", Style::default()),
    ]));

    let block = Block::default()
        .title(" Expanded Line View ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    let area = centered_rect(80, 60, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
