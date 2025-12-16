use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::app::App;
use crate::ui::batch::detect_batches_from_logs;
use crate::ui::filter::FilterType;
use crate::ui::utils::centered_rect;

/// Draw the expanded line view overlay
pub fn draw_expanded_line_overlay(f: &mut Frame, manager: &ProcessManager, app: &App) {
    // Get the selected line ID if available
    let selected_line_id = match app.selected_line_id {
        Some(id) => id,
        None => {
            // No line selected, don't show the overlay
            return;
        }
    };

    // Use snapshot if available (frozen/batch/trace mode), otherwise use live buffer
    // This must match the logic in log_viewer.rs exactly
    let logs_vec: Vec<&LogLine> = if let Some(ref snapshot) = app.snapshot {
        snapshot.iter().collect()
    } else {
        let mut logs = manager.get_all_logs();
        if app.frozen {
            if let Some(frozen_at) = app.frozen_at {
                logs.retain(|log| log.timestamp <= frozen_at);
            }
        }
        logs
    };

    // Apply filters
    let mut filtered_logs: Vec<&LogLine> = if app.filters.is_empty() {
        logs_vec
    } else {
        logs_vec.into_iter()
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

    // Apply search filter if active
    let active_search_pattern = if app.search_mode && !app.input.is_empty() {
        &app.input
    } else if !app.search_pattern.is_empty() {
        &app.search_pattern
    } else {
        ""
    };

    if !active_search_pattern.is_empty() {
        filtered_logs = filtered_logs
            .into_iter()
            .filter(|log| {
                log.line
                    .to_lowercase()
                    .contains(&active_search_pattern.to_lowercase())
            })
            .collect();
    }

    // Apply process visibility filter
    filtered_logs.retain(|log| {
        !app.hidden_processes.contains(log.source.process_name())
    });

    // Apply trace filter mode if active
    if app.trace_filter_mode {
        if let (Some(trace_id), Some(start), Some(end)) = (
            &app.active_trace_id,
            app.trace_time_start,
            app.trace_time_end,
        ) {
            let expanded_start = start - app.trace_expand_before;
            let expanded_end = end + app.trace_expand_after;

            filtered_logs = filtered_logs
                .into_iter()
                .filter(|log| {
                    let contains_trace = log.line.contains(trace_id.as_str());
                    let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
                    contains_trace || (in_time_window && (app.trace_expand_before.num_seconds() > 0 || app.trace_expand_after.num_seconds() > 0))
                })
                .collect();
        }
    }

    // Detect batches
    let batches = detect_batches_from_logs(&filtered_logs, app.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs: Vec<&LogLine> = if app.batch_view_mode {
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

    // Find the selected log by ID
    let (selected_idx, selected_log) = match display_logs.iter().enumerate().find(|(_, log)| log.id == selected_line_id) {
        Some((idx, log)) => (idx, *log),
        None => {
            // Selected line not found in display logs
            return;
        }
    };

    // Find which batch this line belongs to
    let batch_info = if !batches.is_empty() {
        batches.iter().enumerate().find(|(_, (start, end))| {
            selected_idx >= *start && selected_idx <= *end
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
            format!("{} of {}", selected_idx + 1, display_logs.len()),
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
