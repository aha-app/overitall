use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use crate::process::ProcessManager;
use crate::ui::app::App;
use crate::ui::batch::detect_batches_from_logs;
use crate::ui::filter::FilterType;

/// Draw the status bar showing buffer stats and batch info
pub fn draw_status_bar(
    f: &mut Frame,
    area: Rect,
    manager: &ProcessManager,
    app: &App,
) {
    let logs = manager.get_all_logs();

    // Apply filters to get filtered logs
    let filtered_logs: Vec<&crate::log::LogLine> = if app.filters.is_empty() {
        logs
    } else {
        logs.into_iter()
            .filter(|log| {
                let line_text = &log.line;

                for filter in &app.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches(line_text) {
                            return false;
                        }
                    }
                }

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

    // Detect batches from filtered logs
    let batches = detect_batches_from_logs(&filtered_logs, app.batch_window_ms);

    // Build status text with buffer stats and batch info
    let buffer_stats = manager.get_buffer_stats();
    let mut status_parts = vec![
        format!(
            "Buffer: {:.1}/{} MB ({:.0}%) | {} lines",
            buffer_stats.memory_mb,
            buffer_stats.limit_mb,
            buffer_stats.percent,
            buffer_stats.line_count
        )
    ];

    // Add batch info
    if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if batch_idx < batches.len() {
                let (start, end) = batches[batch_idx];
                let line_count = end - start + 1;
                status_parts.push(format!("Batch {}/{}, {} lines", batch_idx + 1, batches.len(), line_count));
            }
        }
    } else if !batches.is_empty() {
        status_parts.push(format!("{} batches", batches.len()));
    }

    let status_text = status_parts.join(" | ");

    let paragraph = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Rgb(40, 40, 40)));

    f.render_widget(paragraph, area);
}
