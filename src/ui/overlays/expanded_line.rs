use ratatui::{
    layout::Rect,
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
use crate::ui::utils::{centered_rect, parse_ansi_to_spans};

/// Context for rendering the expanded line view (shared between overlay and panel)
struct ExpandedLineContext<'a> {
    log: &'a LogLine,
    selected_idx: usize,
    total_logs: usize,
    batch_num: Option<usize>,
}

/// Get the selected log with context for rendering
fn get_selected_log_context<'a>(
    manager: &'a ProcessManager,
    app: &'a App,
) -> Option<ExpandedLineContext<'a>> {
    let selected_line_id = app.navigation.selected_line_id?;

    // Use snapshot if available (frozen/batch/trace mode), otherwise use live buffer
    let logs_vec: Vec<&LogLine> = if let Some(ref snapshot) = app.navigation.snapshot {
        snapshot.iter().collect()
    } else {
        let mut logs = manager.get_all_logs();
        if app.navigation.frozen {
            if let Some(frozen_at) = app.navigation.frozen_at {
                logs.retain(|log| log.timestamp <= frozen_at);
            }
        }
        logs
    };

    // Apply filters
    let mut filtered_logs: Vec<&LogLine> = if app.filters.filters.is_empty() {
        logs_vec
    } else {
        logs_vec
            .into_iter()
            .filter(|log| {
                let line_text = &log.line;

                for filter in &app.filters.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) && filter.matches(line_text)
                    {
                        return false;
                    }
                }

                let include_filters: Vec<_> = app
                    .filters
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
    let active_search_pattern = if app.input.search_mode && !app.input.input.is_empty() {
        &app.input.input
    } else if !app.input.search_pattern.is_empty() {
        &app.input.search_pattern
    } else {
        ""
    };

    if !active_search_pattern.is_empty() {
        let pattern_lower = active_search_pattern.to_lowercase();
        filtered_logs = filtered_logs
            .into_iter()
            .filter(|log| log.line_lowercase().contains(&pattern_lower))
            .collect();
    }

    // Apply process visibility filter
    filtered_logs.retain(|log| !app.filters.hidden_processes.contains(log.source.process_name()));

    // Apply trace filter mode if active
    if app.trace.trace_filter_mode {
        if let (Some(trace_id), Some(start), Some(end)) = (
            &app.trace.active_trace_id,
            app.trace.trace_time_start,
            app.trace.trace_time_end,
        ) {
            let expanded_start = start - app.trace.trace_expand_before;
            let expanded_end = end + app.trace.trace_expand_after;

            filtered_logs = filtered_logs
                .into_iter()
                .filter(|log| {
                    let contains_trace = log.line.contains(trace_id.as_str());
                    let in_time_window =
                        log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
                    contains_trace
                        || (in_time_window
                            && (app.trace.trace_expand_before.num_seconds() > 0
                                || app.trace.trace_expand_after.num_seconds() > 0))
                })
                .collect();
        }
    }

    // Detect batches
    let batches = detect_batches_from_logs(&filtered_logs, app.batch.batch_window_ms);

    // Apply batch view mode filtering if enabled
    let display_logs: Vec<&LogLine> = if app.batch.batch_view_mode {
        if let Some(batch_idx) = app.batch.current_batch {
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
    let (selected_idx, selected_log) = display_logs
        .iter()
        .enumerate()
        .find(|(_, log)| log.id == selected_line_id)?;

    // Find which batch this line belongs to
    let batch_num = if !batches.is_empty() {
        batches
            .iter()
            .enumerate()
            .find(|(_, (start, end))| selected_idx >= *start && selected_idx <= *end)
            .map(|(batch_idx, _)| batch_idx + 1)
    } else {
        None
    };

    Some(ExpandedLineContext {
        log: selected_log,
        selected_idx,
        total_logs: display_logs.len(),
        batch_num,
    })
}

/// Build the content lines for the expanded line view
fn build_expanded_line_content(ctx: &ExpandedLineContext, for_panel: bool) -> Vec<Line<'static>> {
    let mut content = vec![
        Line::from(vec![Span::styled(
            if for_panel {
                "Line Details"
            } else {
                "Expanded Log Line"
            },
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    content.push(Line::from(vec![
        Span::styled("Timestamp: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            ctx.log.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    content.push(Line::from(vec![
        Span::styled("Process: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            ctx.log.source.process_name().to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if let Some(batch_num) = ctx.batch_num {
        content.push(Line::from(vec![
            Span::styled("Batch: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", batch_num), Style::default().fg(Color::Green)),
        ]));
    }

    content.push(Line::from(vec![
        Span::styled("Line: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{} of {}", ctx.selected_idx + 1, ctx.total_logs),
            Style::default().fg(Color::Magenta),
        ),
    ]));

    content.push(Line::from(""));
    content.push(Line::from(vec![Span::styled(
        "Message:",
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    content.push(Line::from(""));

    let parsed_spans = parse_ansi_to_spans(&ctx.log.line);
    let spans: Vec<Span> = parsed_spans
        .into_iter()
        .map(|(text, style)| Span::styled(text, style))
        .collect();
    content.push(Line::from(spans));

    content.push(Line::from(""));

    if for_panel {
        content.push(Line::from(vec![
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::styled(" close | ", Style::default()),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::styled(" copy", Style::default()),
        ]));
    } else {
        content.push(Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::styled(" to close | ", Style::default()),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" to show context | ", Style::default()),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::styled(" to copy", Style::default()),
        ]));
    }

    content
}

/// Draw the expanded line view as a side panel (for wide split-screen mode)
pub fn draw_expanded_line_panel(f: &mut Frame, area: Rect, manager: &ProcessManager, app: &App) {
    let ctx = match get_selected_log_context(manager, app) {
        Some(ctx) => ctx,
        None => {
            // No line selected or not found, draw empty panel
            let block = Block::default()
                .title(" Line Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Select a line to view details",
                    Style::default().fg(Color::DarkGray),
                )]),
            ])
            .block(block);
            f.render_widget(paragraph, area);
            return;
        }
    };

    let content = build_expanded_line_content(&ctx, true);

    let block = Block::default()
        .title(" Line Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the expanded line view overlay
pub fn draw_expanded_line_overlay(f: &mut Frame, manager: &ProcessManager, app: &App) {
    let ctx = match get_selected_log_context(manager, app) {
        Some(ctx) => ctx,
        None => return,
    };

    let content = build_expanded_line_content(&ctx, false);

    let block = Block::default()
        .title(" Expanded Line View ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    let area = centered_rect(80, 60, f.area());

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
