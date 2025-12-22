use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::log::LogLine;
use crate::process::ProcessManager;
use crate::ui::ansi_cache::{AnsiCache, AnsiCacheKey};
use crate::ui::app::App;
use crate::ui::batch_cache::BatchCacheKey;
use crate::ui::filter::FilterType;
use crate::ui::utils::condense_log_line;

/// Draw the log viewer in the middle of the screen
pub fn draw_log_viewer(
    f: &mut Frame,
    area: Rect,
    manager: &ProcessManager,
    app: &mut App,
) {
    // Use snapshot if available (frozen/batch mode), otherwise use live buffer
    let logs_vec: Vec<&LogLine> = if let Some(ref snapshot) = app.snapshot {
        snapshot.iter().collect()
    } else {
        let mut logs = manager.get_all_logs();

        // If display is frozen (without snapshot), only show logs up to the frozen timestamp
        if app.frozen {
            if let Some(frozen_at) = app.frozen_at {
                logs.retain(|log| log.timestamp <= frozen_at);
            }
        }

        logs
    };

    // Apply filters to logs
    let mut filtered_logs: Vec<&LogLine> = if app.filters.is_empty() {
        // No filters, show all logs
        logs_vec
    } else {
        logs_vec.into_iter()
            .filter(|log| {
                let line_lower = log.line_lowercase();

                // Check exclude filters first (if any match, reject the line)
                for filter in &app.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches_lowercase(line_lower) {
                            return false; // Excluded
                        }
                    }
                }

                // Check include filters (if any exist, at least one must match)
                let include_filters: Vec<_> = app
                    .filters
                    .iter()
                    .filter(|f| matches!(f.filter_type, FilterType::Include))
                    .collect();

                if include_filters.is_empty() {
                    return true; // No include filters, line passes
                }

                // At least one include filter must match
                include_filters.iter().any(|filter| filter.matches_lowercase(line_lower))
            })
            .collect()
    };

    // Apply search filter if active (temporary filter)
    // Use app.input if actively typing, otherwise use saved search_pattern
    let active_search_pattern = if app.search_mode && !app.input.is_empty() {
        &app.input
    } else if !app.search_pattern.is_empty() {
        &app.search_pattern
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
            // Calculate expanded time window
            let expanded_start = start - app.trace_expand_before;
            let expanded_end = end + app.trace_expand_after;

            filtered_logs = filtered_logs
                .into_iter()
                .filter(|log| {
                    // Include if: contains trace ID OR is within expanded time window
                    let contains_trace = log.line.contains(trace_id.as_str());
                    let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
                    contains_trace || (in_time_window && (app.trace_expand_before.num_seconds() > 0 || app.trace_expand_after.num_seconds() > 0))
                })
                .collect();
        }
    }

    let match_count = if !active_search_pattern.is_empty() {
        filtered_logs.len()
    } else {
        0
    };

    // Build cache key and detect batches (cached)
    let cache_key = BatchCacheKey::from_context(
        &filtered_logs,
        app.batch_window_ms,
        app.filters.len(),
        active_search_pattern.to_string(),
        app.hidden_processes.len(),
        app.trace_filter_mode,
        app.snapshot.is_some(),
    );
    let batches = app.batch_cache.get_or_compute(&filtered_logs, app.batch_window_ms, cache_key).clone();

    // Update cached batch count for status bar (avoids duplicate batch detection)
    app.cached_batch_count = batches.len();

    // Build a map from each log index to its batch number (before consuming filtered_logs)
    let filtered_log_to_batch: Vec<Option<usize>> = if !batches.is_empty() {
        let mut map = vec![None; filtered_logs.len()];
        for (batch_idx, (start, end)) in batches.iter().enumerate() {
            for i in *start..=*end {
                if i < map.len() {
                    map[i] = Some(batch_idx);
                }
            }
        }
        map
    } else {
        vec![]
    };

    // Validate and adjust current_batch if needed
    let current_batch_validated = if app.batch_view_mode {
        if let Some(batch_idx) = app.current_batch {
            if batch_idx < batches.len() {
                Some(batch_idx)
            } else {
                // current_batch is out of bounds, reset to last batch
                if batches.is_empty() {
                    None
                } else {
                    Some(batches.len() - 1)
                }
            }
        } else {
            // batch_view_mode is on but no batch selected, default to first
            if batches.is_empty() {
                None
            } else {
                Some(0)
            }
        }
    } else {
        None
    };

    // Apply batch view mode filtering if enabled
    let (display_logs_source, display_start_in_filtered): (Vec<&LogLine>, usize) = if let Some(batch_idx) = current_batch_validated {
        if !batches.is_empty() && batch_idx < batches.len() {
            let (start, end) = batches[batch_idx];
            let line_count = end - start + 1;
            // Update cached batch info for status bar
            app.cached_batch_info = Some((batch_idx, batches.len(), line_count));
            (filtered_logs[start..=end].to_vec(), start)
        } else {
            app.cached_batch_info = None;
            (filtered_logs, 0)
        }
    } else {
        app.cached_batch_info = None;
        (filtered_logs, 0)
    };

    // Calculate visible lines
    // Subtract 1 for the title line (Block title takes 1 line even with Borders::NONE)
    let visible_lines = (area.height as usize).saturating_sub(1);
    let total_logs = display_logs_source.len();

    // Find the selected line index by ID (if any line is selected)
    let selected_line_index: Option<usize> = app.selected_line_id.and_then(|id| {
        display_logs_source.iter().position(|log| log.id == id)
    });

    // Determine which logs to display based on scroll state
    let (display_logs, scroll_indicator, display_start) = if app.auto_scroll && selected_line_index.is_none() {
        // Auto-scroll mode: show the last N logs (only when not selecting lines)
        // Account for batch separators: work backwards from the end to find how many logs fit
        let mut start = total_logs;
        let mut lines_used = 0;

        while start > 0 && lines_used < visible_lines {
            start -= 1;
            lines_used += 1; // The log line itself

            // Check if adding this log would add a separator before it
            if start > 0 && current_batch_validated.is_none() && !filtered_log_to_batch.is_empty() {
                let prev_idx = display_start_in_filtered + start - 1;
                let curr_idx = display_start_in_filtered + start;
                let prev_batch = filtered_log_to_batch.get(prev_idx).and_then(|b| *b);
                let curr_batch = filtered_log_to_batch.get(curr_idx).and_then(|b| *b);
                if prev_batch != curr_batch && curr_batch.is_some() {
                    // This log would need a separator before it
                    if lines_used + 1 <= visible_lines {
                        lines_used += 1; // Account for the separator
                    } else {
                        // No room for the separator, don't include this log
                        start += 1;
                        break;
                    }
                }
            }
        }

        let display = &display_logs_source[start..];
        (display, String::new(), start)
    } else if let Some(selected_idx) = selected_line_index {
        // Line selection mode: scroll to show the selected line
        if selected_idx < total_logs {
            // Center the selected line in the viewport for better visibility
            // This gives context both above and below the selection
            let target_position = visible_lines / 3; // Position at 1/3 down (gives more context below)

            let start = if selected_idx < target_position {
                // Selected line is near top - show from beginning
                0
            } else {
                // Position selected line at target_position from top
                selected_idx.saturating_sub(target_position)
            };
            let end = (start + visible_lines).min(total_logs);
            let display = &display_logs_source[start..end];
            (display, String::new(), start)
        } else {
            // Invalid selected index, fall back to manual scroll
            let start = app.scroll_offset.min(total_logs.saturating_sub(visible_lines));
            let end = (start + visible_lines).min(total_logs);
            let display = &display_logs_source[start..end];
            (display, String::new(), start)
        }
    } else {
        // Manual scroll mode: show logs from scroll_offset
        let start = app.scroll_offset.min(total_logs.saturating_sub(visible_lines));
        let end = (start + visible_lines).min(total_logs);
        let display = &display_logs_source[start..end];

        // Calculate scroll position indicator
        let position_pct = if total_logs > 0 {
            (start * 100) / total_logs.max(1)
        } else {
            0
        };
        let indicator = format!(" [{}%] ", position_pct);
        (display, indicator, start)
    };

    // Format log lines: [HH:MM:SS] process_name: message
    // When not in batch view mode, add separators between batches
    let mut log_lines: Vec<Line> = Vec::new();

    for (display_idx, log) in display_logs.iter().enumerate() {
        // Insert batch separator if we're starting a new batch
        // Only show separators when not in batch view mode
        if current_batch_validated.is_none() && display_idx > 0 && !filtered_log_to_batch.is_empty() {
            // Calculate the indices in the filtered_logs array
            // display_start is the offset within display_logs_source
            // display_start_in_filtered is the offset of display_logs_source within filtered_logs
            let prev_filtered_idx = display_start_in_filtered + display_start + display_idx - 1;
            let curr_filtered_idx = display_start_in_filtered + display_start + display_idx;

            // Get batch numbers for previous and current log
            let prev_batch = filtered_log_to_batch.get(prev_filtered_idx).and_then(|b| *b);
            let curr_batch = filtered_log_to_batch.get(curr_filtered_idx).and_then(|b| *b);

            // If we're transitioning to a new batch, insert a separator
            if prev_batch != curr_batch && curr_batch.is_some() {
                let batch_num = curr_batch.unwrap();

                // Get batch info from batches array
                if batch_num < batches.len() {
                    let (batch_start, batch_end) = batches[batch_num];
                    let batch_size = batch_end - batch_start + 1;

                    // Create separator text with batch info
                    let separator_text = format!(" Batch {} ({} logs) ", batch_num + 1, batch_size);
                    let padding_needed = 80_usize.saturating_sub(separator_text.len());
                    let left_padding = padding_needed / 2;
                    let right_padding = padding_needed - left_padding;

                    let separator_line = format!(
                        "{}{}{}",
                        "─".repeat(left_padding),
                        separator_text,
                        "─".repeat(right_padding)
                    );

                    let separator = Line::from(Span::styled(
                        separator_line,
                        Style::default().fg(Color::DarkGray),
                    ));
                    log_lines.push(separator);
                }
            }
        }

        let timestamp = log.timestamp.format("%H:%M:%S").to_string();
        let process_name = log.source.process_name();

        // Check if this line is selected by ID
        let is_selected = app.selected_line_id == Some(log.id);

        // Don't highlight search matches - we're already filtering to show only matches
        // Highlighting would make all visible lines gray, which looks bad
        let is_match = false;

        // Format timestamp and process name parts (no ANSI codes)
        let timestamp_part = format!("[{}] ", timestamp);
        let process_part = format!("{}: ", process_name);

        // Apply condensing in compact mode (but not in batch view mode, which shows full content)
        let log_content = if app.is_compact() && current_batch_validated.is_none() {
            condense_log_line(&log.line)
        } else {
            log.line.clone()
        };

        // Build the full line with ANSI codes preserved
        let full_line_with_ansi = format!("{}{}{}", timestamp_part, process_part, log_content);

        // For width calculations, strip ANSI codes
        let full_line_clean = strip_ansi_escapes::strip_str(&full_line_with_ansi);

        // Calculate max width (account for borders: 2 chars)
        let max_line_width = (area.width as usize).saturating_sub(3); // -2 for borders, -1 for safety

        // Determine if we need to truncate and render accordingly
        let line = if current_batch_validated.is_some() {
            // In batch view mode: show full content with cached ANSI parsing
            let bg_color = if is_selected {
                Some(Color::Blue)
            } else if is_match {
                Some(Color::DarkGray)
            } else {
                None
            };

            let fg_override = if is_selected {
                Some(Color::White)
            } else {
                None
            };

            // Use cache: batch view mode always uses non-compact content
            let cache_key = AnsiCacheKey::new(log.id, false);
            let cached = app.ansi_cache.get_or_parse(cache_key, &full_line_with_ansi);
            AnsiCache::to_line_with_overrides(cached, bg_color, fg_override)
        } else if full_line_clean.width() > max_line_width {
            // Truncate based on display width (using clean text for measurement)
            let mut current_width = 0;
            let mut truncate_at = 0;
            // "… ↵" = 4 display chars (ellipsis + space + return symbol)
            let suffix = "… ↵";
            let suffix_width = suffix.width();
            let target_width = max_line_width.saturating_sub(suffix_width);

            for (idx, ch) in full_line_clean.char_indices() {
                let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                if current_width + char_width > target_width {
                    break;
                }
                current_width += char_width;
                truncate_at = idx + ch.len_utf8();
            }

            // For truncated lines, show hint that Enter expands
            let truncated_text = &full_line_clean[..truncate_at];
            let base_style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else if is_match {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let hint_style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(vec![
                Span::styled(truncated_text.to_string(), base_style),
                Span::styled(suffix, hint_style),
            ])
        } else {
            // Full line fits, parse ANSI codes with caching
            let bg_color = if is_selected {
                Some(Color::Blue)
            } else if is_match {
                Some(Color::DarkGray)
            } else {
                None
            };

            let fg_override = if is_selected {
                Some(Color::White)
            } else {
                None
            };

            // Use cache: key includes compact mode since content may differ
            let cache_key = AnsiCacheKey::new(log.id, app.is_compact());
            let cached = app.ansi_cache.get_or_parse(cache_key, &full_line_with_ansi);
            AnsiCache::to_line_with_overrides(cached, bg_color, fg_override)
        };

        log_lines.push(line);
    }

    // Build title with filters and search info (buffer/batch stats now in status bar)
    let mut title_parts = vec![];

    if app.filter_count() > 0 {
        title_parts.push(format!("({} filters)", app.filter_count()));
    }

    if !active_search_pattern.is_empty() {
        if match_count == 0 {
            title_parts.push(format!("[Search: {}] no matches", active_search_pattern));
        } else {
            title_parts.push(format!("[Search: {}] {} matches", active_search_pattern, match_count));
        }
    }

    if !scroll_indicator.is_empty() {
        title_parts.push(scroll_indicator);
    }

    let title = title_parts.join(" ");

    let mut paragraph = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::NONE)
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    // Enable word wrapping when in batch view mode so full lines are visible
    if current_batch_validated.is_some() {
        paragraph = paragraph.wrap(Wrap { trim: true });
    }

    f.render_widget(paragraph, area);
}
