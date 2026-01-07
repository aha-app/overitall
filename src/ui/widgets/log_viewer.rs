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
use crate::ui::display_state::TimestampMode;
use crate::ui::filter::FilterType;

/// Calculate the display width of a log line (without ANSI codes)
fn calculate_line_width(log: &LogLine, timestamp_mode: TimestampMode, is_compact: bool) -> usize {
    let timestamp_part = match timestamp_mode {
        TimestampMode::Seconds => format!("[{}] ", log.formatted_timestamp()),
        TimestampMode::Milliseconds => format!("[{}] ", log.arrival_time.format("%H:%M:%S%.3f")),
        TimestampMode::Off => String::new(),
    };
    let process_part = format!("{}: ", log.source.process_name());
    let content = if is_compact {
        log.condensed_stripped_line()
    } else {
        log.stripped_line()
    };
    format!("{}{}{}", timestamp_part, process_part, content).width()
}

/// Calculate the number of visual lines a log entry will take when wrapped
fn calculate_wrapped_height(line_width: usize, max_line_width: usize) -> usize {
    if max_line_width == 0 {
        return 1;
    }
    // Ceiling division: (line_width + max_line_width - 1) / max_line_width
    // but at least 1 line
    ((line_width + max_line_width - 1) / max_line_width).max(1)
}

/// Draw the log viewer in the middle of the screen
pub fn draw_log_viewer(
    f: &mut Frame,
    area: Rect,
    manager: &ProcessManager,
    app: &mut App,
) {
    // Use snapshot if available (frozen/batch mode), otherwise use live buffer
    let logs_vec: Vec<&LogLine> = if let Some(ref snapshot) = app.navigation.snapshot {
        snapshot.iter().collect()
    } else {
        let mut logs = manager.get_all_logs();

        // If display is frozen (without snapshot), only show logs up to the frozen timestamp
        if app.navigation.frozen {
            if let Some(frozen_at) = app.navigation.frozen_at {
                logs.retain(|log| log.timestamp <= frozen_at);
            }
        }

        logs
    };

    // Apply filters to logs
    let mut filtered_logs: Vec<&LogLine> = if app.filters.filters.is_empty() {
        // No filters, show all logs
        logs_vec
    } else {
        logs_vec.into_iter()
            .filter(|log| {
                let line_lower = log.line_lowercase();

                // Check exclude filters first (if any match, reject the line)
                for filter in &app.filters.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches_lowercase(line_lower) {
                            return false; // Excluded
                        }
                    }
                }

                // Check include filters (if any exist, at least one must match)
                let include_filters: Vec<_> = app
                    .filters
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
    // Skip search filter in batch view mode - batch view shows raw batch content
    let active_search_pattern = if app.batch.batch_view_mode {
        // In batch view, show raw logs without search filtering
        ""
    } else if app.input.search_mode && !app.input.input.is_empty() {
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
    filtered_logs.retain(|log| {
        !app.filters.hidden_processes.contains(log.source.process_name())
    });

    // Apply trace filter mode if active
    if app.trace.trace_filter_mode {
        if let (Some(trace_id), Some(start), Some(end)) = (
            &app.trace.active_trace_id,
            app.trace.trace_time_start,
            app.trace.trace_time_end,
        ) {
            // Calculate expanded time window
            let expanded_start = start - app.trace.trace_expand_before;
            let expanded_end = end + app.trace.trace_expand_after;

            filtered_logs = filtered_logs
                .into_iter()
                .filter(|log| {
                    // Include if: contains trace ID OR is within expanded time window
                    let contains_trace = log.line.contains(trace_id.as_str());
                    let in_time_window = log.arrival_time >= expanded_start && log.arrival_time <= expanded_end;
                    contains_trace || (in_time_window && (app.trace.trace_expand_before.num_seconds() > 0 || app.trace.trace_expand_after.num_seconds() > 0))
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
        app.batch.batch_window_ms,
        app.filters.filters.len(),
        active_search_pattern.to_string(),
        app.filters.hidden_processes.len(),
        app.trace.trace_filter_mode,
        app.navigation.snapshot.is_some(),
    );
    let batches = app.cache.batch_cache.get_or_compute(&filtered_logs, app.batch.batch_window_ms, cache_key).clone();

    // Update cached batch count for status bar (avoids duplicate batch detection)
    app.cache.cached_batch_count = batches.len();

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
    let current_batch_validated = if app.batch.batch_view_mode {
        if let Some(batch_idx) = app.batch.current_batch {
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
            app.cache.cached_batch_info = Some((batch_idx, batches.len(), line_count));
            (filtered_logs[start..=end].to_vec(), start)
        } else {
            app.cache.cached_batch_info = None;
            (filtered_logs, 0)
        }
    } else {
        app.cache.cached_batch_info = None;
        (filtered_logs, 0)
    };

    // Calculate visible lines
    // Subtract 1 for the title line (Block title takes 1 line even with Borders::NONE)
    let visible_lines = (area.height as usize).saturating_sub(1);
    let total_logs = display_logs_source.len();

    // Calculate max line width for wrap mode height calculations
    // Account for borders: 2 chars, plus 1 for safety
    let max_line_width = (area.width as usize).saturating_sub(3);

    // Check if we're in wrap mode (affects scroll calculations)
    let is_wrap_mode = current_batch_validated.is_some() || app.display.is_wrap();

    // In wrap mode, we need to know how many visual lines each log takes
    // Pre-calculate wrapped heights for wrap mode to avoid repeated calculations
    let wrapped_heights: Vec<usize> = if is_wrap_mode && total_logs > 0 {
        display_logs_source
            .iter()
            .map(|log| {
                // In batch/wrap mode, we show full content (not condensed)
                let line_width = calculate_line_width(log, app.display.timestamp_mode, false);
                calculate_wrapped_height(line_width, max_line_width)
            })
            .collect()
    } else {
        vec![]
    };

    // Find the selected line index by ID (if any line is selected)
    let selected_line_index: Option<usize> = app.navigation.selected_line_id.and_then(|id| {
        display_logs_source.iter().position(|log| log.id == id)
    });

    // Determine which logs to display based on scroll state
    let (display_logs, scroll_indicator, display_start) = if app.navigation.auto_scroll && selected_line_index.is_none() {
        // Auto-scroll mode: show the last N logs (only when not selecting lines)
        // Account for batch separators: work backwards from the end to find how many logs fit
        let mut start = total_logs;
        let mut lines_used = 0;

        while start > 0 && lines_used < visible_lines {
            start -= 1;

            // In wrap mode, use the pre-calculated wrapped height
            let log_height = if is_wrap_mode {
                wrapped_heights.get(start).copied().unwrap_or(1)
            } else {
                1
            };

            // Check if we have room for this log
            if lines_used + log_height > visible_lines {
                // No room for this log, undo and stop
                start += 1;
                break;
            }
            lines_used += log_height;

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
            if is_wrap_mode {
                // In wrap mode: calculate positions based on wrapped heights
                // We want the selected line fully visible, positioned about 1/3 down
                let selected_height = wrapped_heights.get(selected_idx).copied().unwrap_or(1);

                // Calculate how many visual lines are above the selected line
                let visual_lines_before: usize = wrapped_heights[..selected_idx].iter().sum();

                // Target: position selected line about 1/3 down from top
                let target_visual_position = visible_lines / 3;

                // Find the starting log index that achieves this positioning
                let mut start = if visual_lines_before < target_visual_position {
                    // Selected line is near the top visually - start from beginning
                    0
                } else {
                    // Work backwards from selected_idx to find which log to start at
                    let target_start_visual = visual_lines_before.saturating_sub(target_visual_position);
                    let mut cumulative = 0;
                    let mut start_idx = 0;
                    for (i, &height) in wrapped_heights.iter().enumerate() {
                        if cumulative >= target_start_visual {
                            start_idx = i;
                            break;
                        }
                        cumulative += height;
                        start_idx = i + 1;
                    }
                    start_idx.min(selected_idx)
                };

                // Calculate end: show as many logs as fit, ensuring selected line is visible
                let mut end = start;
                let mut lines_used = 0;
                while end < total_logs && lines_used < visible_lines {
                    let height = wrapped_heights.get(end).copied().unwrap_or(1);
                    if end > selected_idx && lines_used + height > visible_lines {
                        // We've shown the selected line and can't fit more
                        break;
                    }
                    // Must include at least up to selected_idx
                    if end <= selected_idx || lines_used + height <= visible_lines {
                        lines_used += height;
                        end += 1;
                    } else {
                        break;
                    }
                }

                // Ensure selected line is visible by adjusting start if needed
                if end <= selected_idx {
                    // Selected line isn't included, adjust to show it
                    end = selected_idx + 1;
                    // Recalculate start to fit
                    let mut lines_needed: usize = wrapped_heights[..end].iter().skip(start).sum();
                    while start < selected_idx && lines_needed + selected_height > visible_lines {
                        lines_needed -= wrapped_heights.get(start).copied().unwrap_or(1);
                        start += 1;
                    }
                }

                let display = &display_logs_source[start..end.min(total_logs)];
                (display, String::new(), start)
            } else {
                // Non-wrap mode: simple line-based positioning
                let target_position = visible_lines / 3;

                let start = if selected_idx < target_position {
                    0
                } else {
                    selected_idx.saturating_sub(target_position)
                };
                let end = (start + visible_lines).min(total_logs);
                let display = &display_logs_source[start..end];
                (display, String::new(), start)
            }
        } else {
            // Invalid selected index, fall back to manual scroll
            let start = app.navigation.scroll_offset.min(total_logs.saturating_sub(visible_lines));
            let end = (start + visible_lines).min(total_logs);
            let display = &display_logs_source[start..end];
            (display, String::new(), start)
        }
    } else {
        // Manual scroll mode: show logs from scroll_offset
        let start = app.navigation.scroll_offset.min(total_logs.saturating_sub(visible_lines));
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

        let process_name = log.source.process_name();

        // Check if this line is the cursor (selected by ID)
        let is_cursor = app.navigation.selected_line_id == Some(log.id);

        // Check if this line is in a multi-select range (but not the cursor itself)
        let is_multi_selected = !is_cursor
            && app.navigation.has_multi_select()
            && app.navigation.is_in_selection_ref(log.id, display_logs);

        // Combined selection state (will be used by Step 6 for clearing on Escape)
        let _is_selected = is_cursor || is_multi_selected;

        // Format timestamp based on mode
        let timestamp_part = match app.display.timestamp_mode {
            TimestampMode::Seconds => format!("[{}] ", log.formatted_timestamp()),
            TimestampMode::Milliseconds => format!("[{}] ", log.arrival_time.format("%H:%M:%S%.3f")),
            TimestampMode::Off => String::new(),
        };

        // Get color ANSI codes for this process/log file name
        let (color_start, color_reset) = app.process_colors.get_ansi(process_name);

        // Process part with ANSI color codes for cached rendering paths
        let process_part_colored = format!("{}{}{}: ", color_start, process_name, color_reset);
        // Process part without color for width calculations
        let process_part_plain = format!("{}: ", process_name);

        // Apply condensing in compact mode (but not in batch view mode, which shows full content)
        // Use cached condensed line
        let (log_content, log_content_stripped): (&str, &str) = if app.display.is_compact() && current_batch_validated.is_none() {
            (log.condensed_line(), log.condensed_stripped_line())
        } else {
            (&log.line, log.stripped_line())
        };

        // Build the full line with ANSI codes preserved (includes colored process name)
        let full_line_with_ansi = format!("{}{}{}", timestamp_part, process_part_colored, log_content);

        // For width calculations, use cached stripped content (no ANSI codes)
        let full_line_clean = format!("{}{}{}", timestamp_part, process_part_plain, log_content_stripped);

        // Determine if we need to truncate and render accordingly
        let line = if current_batch_validated.is_some() || app.display.is_wrap() {
            // In batch view mode or wrap mode: show full content with cached ANSI parsing
            // Paragraph wrapping is applied at the widget level
            let bg_color = if is_cursor {
                Some(Color::Blue)
            } else if is_multi_selected {
                Some(Color::Rgb(30, 50, 70))
            } else {
                None
            };

            let fg_override = if is_cursor {
                Some(Color::White)
            } else {
                None
            };

            // Use cache: batch/wrap view mode always uses non-compact content
            let cache_key = AnsiCacheKey::new(log.id, false, app.display.timestamp_mode);
            let cached = app.cache.ansi_cache.get_or_parse(cache_key, &full_line_with_ansi);
            AnsiCache::to_line_with_overrides(cached, bg_color, fg_override)
        } else if full_line_clean.width() > max_line_width {
            // Truncate with ANSI color preservation
            let suffix = "… ↵";
            let suffix_width = suffix.width();
            let target_width = max_line_width.saturating_sub(suffix_width);

            let bg_color = if is_cursor {
                Some(Color::Blue)
            } else if is_multi_selected {
                Some(Color::Rgb(30, 50, 70))
            } else {
                None
            };

            let fg_override = if is_cursor {
                Some(Color::White)
            } else {
                None
            };

            let hint_style = if is_cursor {
                Style::default().fg(Color::Cyan)
            } else if is_multi_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Use cached ANSI parsing and truncate the spans
            let cache_key = AnsiCacheKey::new(log.id, app.display.is_compact(), app.display.timestamp_mode);
            let cached = app.cache.ansi_cache.get_or_parse(cache_key, &full_line_with_ansi);
            AnsiCache::to_truncated_line(cached, target_width, bg_color, fg_override, suffix, hint_style)
        } else {
            // Full line fits, parse ANSI codes with caching
            let bg_color = if is_cursor {
                Some(Color::Blue)
            } else if is_multi_selected {
                Some(Color::Rgb(30, 50, 70))
            } else {
                None
            };

            let fg_override = if is_cursor {
                Some(Color::White)
            } else {
                None
            };

            // Use cache: key includes compact mode and timestamp mode since content may differ
            let cache_key = AnsiCacheKey::new(log.id, app.display.is_compact(), app.display.timestamp_mode);
            let cached = app.cache.ansi_cache.get_or_parse(cache_key, &full_line_with_ansi);
            AnsiCache::to_line_with_overrides(cached, bg_color, fg_override)
        };

        log_lines.push(line);
    }

    // Build title with filters and search info (buffer/batch stats now in status bar)
    let mut title_parts = vec![];

    if app.filters.filter_count() > 0 {
        title_parts.push(format!("({} filters)", app.filters.filter_count()));
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

    // Enable word wrapping in batch view mode or wrap mode so full lines are visible
    if current_batch_validated.is_some() || app.display.is_wrap() {
        paragraph = paragraph.wrap(Wrap { trim: true });
    }

    f.render_widget(paragraph, area);
}
