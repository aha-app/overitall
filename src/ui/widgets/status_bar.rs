use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::process::ProcessManager;
use crate::ui::app::App;

/// Draw the status bar showing buffer stats and batch info
///
/// Uses cached batch info from log_viewer to avoid duplicate O(n) batch detection.
pub fn draw_status_bar(
    f: &mut Frame,
    area: Rect,
    manager: &ProcessManager,
    app: &App,
) {
    // Build status text with buffer stats and batch info
    let buffer_stats = manager.get_buffer_stats();
    let mut status_parts = vec![
        format!(
            "Buffer: {:.1}/{} MB ({:.0}%) | {} lines {}",
            buffer_stats.memory_mb,
            buffer_stats.limit_mb,
            buffer_stats.percent,
            buffer_stats.line_count,
            buffer_stats.sparkline
        )
    ];

    // Add batch info (using cached values from log_viewer)
    if app.batch.batch_view_mode {
        if let Some((batch_idx, total_batches, line_count)) = app.cache.cached_batch_info {
            status_parts.push(format!("Batch {}/{}, {} lines", batch_idx + 1, total_batches, line_count));
        }
    } else if app.cache.cached_batch_count > 0 {
        status_parts.push(format!("{} batches", app.cache.cached_batch_count));
    }

    let status_text = status_parts.join(" | ");

    // Mode/scroll indicator - show mode when in a special view, otherwise show tail/scroll state
    let mode_indicator = if app.trace.trace_filter_mode {
        // In trace view - logs are frozen to a specific trace
        Span::styled(
            "[TRACE]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )
    } else if app.batch.batch_view_mode {
        // In batch view - viewing a specific batch
        Span::styled(
            "[BATCH]",
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        )
    } else if app.navigation.auto_scroll {
        // Normal mode, following new logs
        Span::styled(
            "[TAIL]",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        )
    } else {
        // Normal mode, viewing history
        Span::styled(
            "[SCROLL]",
            Style::default().fg(Color::Yellow)
        )
    };

    // Build styled line with optional recording indicator and scroll state
    let line = if app.trace.manual_trace_recording {
        // Show red recording indicator with elapsed time
        let elapsed_secs = app.trace.manual_trace_start
            .map(|start| (Local::now() - start).num_seconds())
            .unwrap_or(0);

        let rec_span = Span::styled(
            format!("● REC {}s ", elapsed_secs),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        );
        let status_span = Span::raw(format!("{} ", status_text));
        Line::from(vec![rec_span, status_span, mode_indicator])
    } else {
        Line::from(vec![Span::raw(format!("{} ", status_text)), mode_indicator])
    };

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(Color::Rgb(40, 40, 40)));

    f.render_widget(paragraph, area);
}
