use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::process::{ProcessManager, ProcessStatus};

/// Type of status message
#[derive(Debug, Clone)]
pub enum StatusType {
    Success,
    Error,
    Info,
}

/// Filter type
#[derive(Debug, Clone)]
pub enum FilterType {
    Include,
    Exclude,
}

/// A log filter
#[derive(Debug, Clone)]
pub struct Filter {
    pub pattern: String,
    pub filter_type: FilterType,
    pub is_regex: bool, // For future: support both plain text and regex
}

impl Filter {
    /// Create a new filter (start with plain text matching)
    pub fn new(pattern: String, filter_type: FilterType) -> Self {
        Self {
            pattern,
            filter_type,
            is_regex: false, // Start with plain text, add regex support later
        }
    }

    pub fn matches(&self, line: &str) -> bool {
        if self.is_regex {
            // Future: regex matching
            false
        } else {
            // Plain text substring matching (case-insensitive)
            line.to_lowercase().contains(&self.pattern.to_lowercase())
        }
    }
}

/// Application state for the TUI
pub struct App {
    /// Current command input text
    pub input: String,
    /// Scroll offset for log viewer (lines from top)
    pub scroll_offset: usize,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Whether to auto-scroll to bottom (stick to latest logs)
    pub auto_scroll: bool,
    /// Whether we're in command mode (user is typing a command)
    pub command_mode: bool,
    /// Status message to show to the user (message, type)
    pub status_message: Option<(String, StatusType)>,
    /// Command history for Up/Down navigation
    pub command_history: Vec<String>,
    /// Current position in history (None = not navigating)
    pub history_index: Option<usize>,
    /// Active log filters
    pub filters: Vec<Filter>,
    /// Whether we're in search mode (user is typing a search)
    pub search_mode: bool,
    /// Current search pattern
    pub search_pattern: String,
    /// Index of current search match
    pub current_match: Option<usize>,
    /// Time window for batch detection in milliseconds
    pub batch_window_ms: i64,
    /// If true, show only the current batch
    pub batch_view_mode: bool,
    /// Index of currently viewed batch
    pub current_batch: Option<usize>,
    /// Whether to show the help overlay
    pub show_help: bool,
    /// Index of selected line (for line expansion/clipboard)
    pub selected_line_index: Option<usize>,
    /// Whether to show expanded line view
    pub expanded_line_view: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            scroll_offset: 0,
            should_quit: false,
            auto_scroll: true, // Start with auto-scroll enabled
            command_mode: false,
            status_message: None,
            command_history: Vec::new(),
            history_index: None,
            filters: Vec::new(),
            search_mode: false,
            search_pattern: String::new(),
            current_match: None,
            batch_window_ms: 100,
            batch_view_mode: false,
            current_batch: None,
            show_help: false,
            selected_line_index: None,
            expanded_line_view: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, lines: usize, max_offset: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
        // If we scrolled to the bottom, re-enable auto-scroll
        if self.scroll_offset >= max_offset {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    /// Jump to bottom and enable auto-scroll
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll_offset = 0; // Will be recalculated when auto_scroll is true
    }

    pub fn enter_command_mode(&mut self) {
        self.command_mode = true;
        self.input.clear();
        self.status_message = None; // Clear status when entering command mode
        self.history_index = None; // Reset history navigation
    }

    pub fn exit_command_mode(&mut self) {
        self.command_mode = false;
        self.input.clear();
    }

    /// Add a character to the command or search input
    pub fn add_char(&mut self, c: char) {
        if self.command_mode {
            self.reset_history_nav(); // Stop navigating history when user types
            self.input.push(c);
        } else if self.search_mode {
            self.input.push(c);
        }
    }

    /// Delete the last character from the command or search input
    pub fn delete_char(&mut self) {
        if self.command_mode || self.search_mode {
            self.input.pop();
        }
    }

    pub fn set_status_success(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Success));
    }

    pub fn set_status_error(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Error));
    }

    pub fn set_status_info(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Info));
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn save_to_history(&mut self, command: String) {
        if !command.is_empty() {
            self.command_history.push(command);
        }
    }

    /// Navigate backward in history (Up arrow)
    pub fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1, // Start at most recent
            Some(0) => 0,                            // Already at oldest, stay there
            Some(i) => i - 1,                        // Go back one
        };

        self.history_index = Some(new_index);
        self.input = self.command_history[new_index].clone();
    }

    /// Navigate forward in history (Down arrow)
    pub fn history_next(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Not navigating history, do nothing
            }
            Some(i) if i >= self.command_history.len() - 1 => {
                // At newest entry, clear input and exit history mode
                self.history_index = None;
                self.input.clear();
            }
            Some(i) => {
                // Go forward one
                let new_index = i + 1;
                self.history_index = Some(new_index);
                self.input = self.command_history[new_index].clone();
            }
        }
    }

    /// Reset history navigation (call when user starts typing)
    pub fn reset_history_nav(&mut self) {
        self.history_index = None;
    }

    pub fn add_include_filter(&mut self, pattern: String) {
        self.filters.push(Filter::new(pattern, FilterType::Include));
    }

    pub fn add_exclude_filter(&mut self, pattern: String) {
        self.filters.push(Filter::new(pattern, FilterType::Exclude));
    }

    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.input.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.input.clear();
    }

    pub fn perform_search(&mut self, pattern: String) {
        self.search_pattern = pattern;
        self.current_match = Some(0);
    }

    pub fn clear_search(&mut self) {
        self.search_pattern.clear();
        self.current_match = None;
    }

    pub fn next_match(&mut self, total_matches: usize) {
        if total_matches == 0 {
            return;
        }
        if let Some(idx) = self.current_match {
            self.current_match = Some((idx + 1) % total_matches);
        }
    }

    pub fn prev_match(&mut self, total_matches: usize) {
        if total_matches == 0 {
            return;
        }
        if let Some(idx) = self.current_match {
            if idx > 0 {
                self.current_match = Some(idx - 1);
            } else {
                self.current_match = Some(total_matches - 1);
            }
        }
    }

    pub fn next_batch(&mut self) {
        if let Some(current) = self.current_batch {
            self.current_batch = Some(current + 1);
        } else {
            self.current_batch = Some(0);
        }
        self.batch_view_mode = true;
        self.scroll_offset = 0;  // Reset scroll to top of batch
        self.auto_scroll = false; // Disable auto-scroll
    }

    pub fn prev_batch(&mut self) {
        if let Some(current) = self.current_batch {
            if current > 0 {
                self.current_batch = Some(current - 1);
            }
        }
        self.batch_view_mode = true;
        self.scroll_offset = 0;  // Reset scroll to top of batch
        self.auto_scroll = false; // Disable auto-scroll
    }

    pub fn toggle_batch_view(&mut self) {
        self.batch_view_mode = !self.batch_view_mode;
        if !self.batch_view_mode {
            self.current_batch = None;
        } else if self.current_batch.is_none() {
            self.current_batch = Some(0);
        }
    }

    pub fn set_batch_window(&mut self, window_ms: i64) {
        self.batch_window_ms = window_ms;
        // Reset batch view to avoid invalid batch indices
        if self.batch_view_mode {
            self.current_batch = Some(0);
            self.scroll_offset = 0;
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn select_line(&mut self, index: Option<usize>) {
        self.selected_line_index = index;
    }

    pub fn select_next_line(&mut self, max_lines: usize) {
        if max_lines == 0 {
            return;
        }
        self.selected_line_index = Some(match self.selected_line_index {
            None => 0,
            Some(idx) if idx >= max_lines - 1 => 0, // Wrap to top when at bottom
            Some(idx) => idx + 1,
        });
        self.auto_scroll = false;
    }

    pub fn select_prev_line(&mut self, max_lines: usize) {
        if max_lines == 0 {
            return;
        }
        self.selected_line_index = Some(match self.selected_line_index {
            None => 0,
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => max_lines - 1, // Wrap to bottom when at top
        });
        self.auto_scroll = false;
    }

    pub fn toggle_expanded_view(&mut self) {
        self.expanded_line_view = !self.expanded_line_view;
    }

    pub fn close_expanded_view(&mut self) {
        self.expanded_line_view = false;
    }
}

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &App, manager: &ProcessManager) {
    // Create the main layout: process list, log viewer, command input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10), // Process list
            Constraint::Min(0),         // Log viewer (takes remaining space)
            Constraint::Length(3),      // Command input (exactly 3 lines)
        ])
        .split(f.area());

    // Draw process list
    draw_process_list(f, chunks[0], manager);

    // Draw log viewer
    draw_log_viewer(f, chunks[1], manager, app);

    // Draw command input
    draw_command_input(f, chunks[2], app);

    // Draw help overlay if show_help is true (must be last so it's on top)
    if app.show_help {
        draw_help_overlay(f);
    }

    // Draw expanded line view overlay if enabled (must be last so it's on top)
    if app.expanded_line_view {
        draw_expanded_line_overlay(f, manager, app);
    }
}

/// Detect batches from a slice of LogLine references
/// Returns a vector of (start_index, end_index) tuples for each batch
pub fn detect_batches_from_logs(logs: &[&crate::log::LogLine], window_ms: i64) -> Vec<(usize, usize)> {
    if logs.is_empty() {
        return vec![];
    }

    if logs.len() == 1 {
        return vec![(0, 0)];
    }

    let mut batches = Vec::new();
    let mut batch_start = 0;

    for i in 1..logs.len() {
        // Compare to the start of the current batch, not the previous log
        // This prevents "chaining" where logs slowly drift apart over time
        let time_diff = logs[i].arrival_time - logs[batch_start].arrival_time;
        if time_diff.num_milliseconds() > window_ms {
            batches.push((batch_start, i - 1));
            batch_start = i;
        }
    }

    batches.push((batch_start, logs.len() - 1));
    batches
}

/// Draw the process list at the top of the screen
fn draw_process_list(f: &mut Frame, area: ratatui::layout::Rect, manager: &ProcessManager) {
    let mut processes = manager.get_all_statuses();

    // Sort processes by name for consistent display
    processes.sort_by(|a, b| a.0.cmp(&b.0));

    // Build a horizontal layout of processes with separators
    let mut spans = Vec::new();

    for (i, (name, status)) in processes.iter().enumerate() {
        // Add separator between processes (but not before the first one)
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }

        let (status_text, color) = match status {
            ProcessStatus::Running => ("Running", Color::Green),
            ProcessStatus::Stopped => ("Stopped", Color::Yellow),
            ProcessStatus::Failed(_) => ("Failed", Color::Red),
        };

        // Add process name and status
        spans.push(Span::styled(
            name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" ["));
        spans.push(Span::styled(status_text, Style::default().fg(color)));
        spans.push(Span::raw("]"));
    }

    // If no processes, show a message
    if spans.is_empty() {
        spans.push(Span::styled(
            "No processes",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Create a single line with all processes
    let line = Line::from(spans);

    // Wrap into a paragraph that can handle text wrapping if needed
    let paragraph = Paragraph::new(vec![line])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Processes ")
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Draw the log viewer in the middle of the screen
fn draw_log_viewer(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    manager: &ProcessManager,
    app: &App,
) {
    let logs = manager.get_all_logs();

    // Apply filters to logs
    let filtered_logs: Vec<&crate::log::LogLine> = if app.filters.is_empty() {
        // No filters, show all logs
        logs
    } else {
        logs.into_iter()
            .filter(|log| {
                let line_text = &log.line;

                // Check exclude filters first (if any match, reject the line)
                for filter in &app.filters {
                    if matches!(filter.filter_type, FilterType::Exclude) {
                        if filter.matches(line_text) {
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
                include_filters.iter().any(|filter| filter.matches(line_text))
            })
            .collect()
    };

    // Detect batches from filtered logs
    let batches = detect_batches_from_logs(&filtered_logs, app.batch_window_ms);

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
    let (display_logs_source, display_start_in_filtered): (Vec<&crate::log::LogLine>, usize) = if let Some(batch_idx) = current_batch_validated {
        if !batches.is_empty() && batch_idx < batches.len() {
            let (start, end) = batches[batch_idx];
            (filtered_logs[start..=end].to_vec(), start)
        } else {
            (filtered_logs, 0)
        }
    } else {
        (filtered_logs, 0)
    };

    // Calculate visible lines (area height minus 2 for borders)
    let visible_lines = area.height.saturating_sub(2) as usize;
    let total_logs = display_logs_source.len();

    // Find search matches
    let search_matches: Vec<usize> = if !app.search_pattern.is_empty() {
        display_logs_source
            .iter()
            .enumerate()
            .filter(|(_, log)| {
                log.line
                    .to_lowercase()
                    .contains(&app.search_pattern.to_lowercase())
            })
            .map(|(idx, _)| idx)
            .collect()
    } else {
        Vec::new()
    };

    let total_matches = search_matches.len();

    // Determine which logs to display based on scroll state
    let (display_logs, scroll_indicator, display_start) = if app.auto_scroll && app.current_match.is_none() {
        // Auto-scroll mode: show the last N logs (only when not navigating search)
        let start = total_logs.saturating_sub(visible_lines);
        let display = &display_logs_source[start..];
        (display, String::new(), start)
    } else if let Some(match_idx) = app.current_match {
        // Search mode: scroll to show the current match
        if match_idx < total_matches {
            let log_idx = search_matches[match_idx];
            // Center the match in the viewport
            let start = if log_idx < visible_lines / 2 {
                0
            } else {
                (log_idx - visible_lines / 2).min(total_logs.saturating_sub(visible_lines))
            };
            let end = (start + visible_lines).min(total_logs);
            let display = &display_logs_source[start..end];
            (display, String::new(), start)
        } else {
            // Invalid match index, fall back to manual scroll
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

        // Check if this line is a search match and if it's the current match
        let log_global_idx = display_start + display_idx;
        let is_match = search_matches.contains(&log_global_idx);
        let is_current_match = if let Some(match_idx) = app.current_match {
            match_idx < total_matches && search_matches.get(match_idx) == Some(&log_global_idx)
        } else {
            false
        };

        // Check if this line is selected
        let is_selected = app.selected_line_index == Some(log_global_idx);

        // Calculate max width for the message part
        // Format: [HH:MM:SS] process_name: message
        // Reserve space for timestamp [HH:MM:SS] (11 chars) + process name + ": " + some margin
        let prefix_len = 11 + process_name.len() + 2;
        let available_width = (area.width as usize).saturating_sub(prefix_len + 4); // 4 for borders and margin

        // Choose style based on whether this is selected, a match, etc.
        let line_style = if is_selected {
            // Selected line: blue background
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if is_current_match {
            // Current match: yellow background
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else if is_match {
            // Other matches: dark gray background
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        // Build the message span, truncating if needed (but not in batch view mode)
        let message_span = if current_batch_validated.is_some() {
            // In batch view mode: show full content (no truncation)
            Span::styled(log.line.clone(), line_style)
        } else if log.line.len() > available_width {
            // Not in batch view mode: truncate long lines
            let truncated = format!("{}...", &log.line[..available_width.saturating_sub(3)]);
            Span::styled(truncated, line_style)
        } else {
            Span::styled(log.line.clone(), line_style)
        };

        let line = Line::from(vec![
            Span::styled(
                format!("[{}] ", timestamp),
                if is_selected { Style::default().bg(Color::Blue).fg(Color::Cyan) } else { Style::default().fg(Color::Cyan) },
            ),
            Span::styled(
                format!("{}: ", process_name),
                if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                },
            ),
            message_span,
        ]);
        log_lines.push(line);
    }

    // Build title with batch count, filters and search info
    let mut title_parts = vec![" Logs ".to_string()];

    // Add batch count if batches exist
    if let Some(batch_idx) = current_batch_validated {
        // In batch view mode with a valid batch selected
        // Calculate the number of lines in this batch
        if let Some(&(start, end)) = batches.get(batch_idx) {
            let line_count = end - start + 1;
            title_parts.push(format!("(Batch {}/{}, {} lines)", batch_idx + 1, batches.len(), line_count));
        } else {
            // Fallback if batch lookup fails
            title_parts.push(format!("(Batch {}/{})", batch_idx + 1, batches.len()));
        }
    } else if !batches.is_empty() {
        // Not in batch view mode, show total batch count
        title_parts.push(format!("({} batches)", batches.len()));
    }

    if app.filter_count() > 0 {
        title_parts.push(format!("({} filters)", app.filter_count()));
    }

    if !app.search_pattern.is_empty() {
        if total_matches == 0 {
            title_parts.push("[Search: no matches]".to_string());
        } else if let Some(match_idx) = app.current_match {
            title_parts.push(format!("[Search: {} of {}]", match_idx + 1, total_matches));
        } else {
            title_parts.push(format!("[Search: {} matches]", total_matches));
        }
    }

    if !scroll_indicator.is_empty() {
        title_parts.push(scroll_indicator);
    }

    let title = title_parts.join(" ");

    let mut paragraph = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    // Enable word wrapping when in batch view mode so full lines are visible
    if current_batch_validated.is_some() {
        use ratatui::widgets::Wrap;
        paragraph = paragraph.wrap(Wrap { trim: true });
    }

    f.render_widget(paragraph, area);
}

/// Draw the command input at the bottom of the screen
fn draw_command_input(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let text = if app.search_mode {
        // Show search input with a cursor
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::raw(&app.input),
            Span::styled("_", Style::default().fg(Color::Cyan)),
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

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

/// Helper function to create a centered rect using percentage of the available area
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Draw the help overlay
fn draw_help_overlay(f: &mut Frame) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Modifier, Style};

    let help_text = vec![
        Line::from(vec![
            Span::styled("Overitall Help", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw("     Select previous/next log line"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow)),
            Span::raw("   Expand selected line (show full content)"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Yellow)),
            Span::raw("     Jump to latest logs (reset view)"),
        ]),
        Line::from(vec![
            Span::styled("  q", Style::default().fg(Color::Yellow)),
            Span::raw("       Quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Commands:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :", Style::default().fg(Color::Yellow)),
            Span::raw("       Enter command mode"),
        ]),
        Line::from(vec![
            Span::styled("  :s <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Start process"),
        ]),
        Line::from(vec![
            Span::styled("  :r <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Restart process"),
        ]),
        Line::from(vec![
            Span::styled("  :k <proc>", Style::default().fg(Color::Yellow)),
            Span::raw(" Kill process"),
        ]),
        Line::from(vec![
            Span::styled("  :q", Style::default().fg(Color::Yellow)),
            Span::raw("       Quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Filtering:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  :f <pat>", Style::default().fg(Color::Yellow)),
            Span::raw("  Include filter (show only matching lines)"),
        ]),
        Line::from(vec![
            Span::styled("  :fn <pat>", Style::default().fg(Color::Yellow)),
            Span::raw(" Exclude filter (hide matching lines)"),
        ]),
        Line::from(vec![
            Span::styled("  :fc", Style::default().fg(Color::Yellow)),
            Span::raw("       Clear all filters"),
        ]),
        Line::from(vec![
            Span::styled("  :fl", Style::default().fg(Color::Yellow)),
            Span::raw("       List active filters"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Search:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Yellow)),
            Span::raw("       Enter search mode"),
        ]),
        Line::from(vec![
            Span::styled("  n", Style::default().fg(Color::Yellow)),
            Span::raw("       Next search match"),
        ]),
        Line::from(vec![
            Span::styled("  N", Style::default().fg(Color::Yellow)),
            Span::raw("       Previous search match"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Batch Navigation:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::Yellow)),
            Span::raw("       Previous batch"),
        ]),
        Line::from(vec![
            Span::styled("  ]", Style::default().fg(Color::Yellow)),
            Span::raw("       Next batch"),
        ]),
        Line::from(vec![
            Span::styled("  :pb", Style::default().fg(Color::Yellow)),
            Span::raw("      Previous batch (same as [)"),
        ]),
        Line::from(vec![
            Span::styled("  :nb", Style::default().fg(Color::Yellow)),
            Span::raw("      Next batch (same as ])"),
        ]),
        Line::from(vec![
            Span::styled("  :sb", Style::default().fg(Color::Yellow)),
            Span::raw("      Toggle batch view mode"),
        ]),
        Line::from(vec![
            Span::styled("  :bw", Style::default().fg(Color::Yellow)),
            Span::raw("       Show current batch window"),
        ]),
        Line::from(vec![
            Span::styled("  :bw <ms>", Style::default().fg(Color::Yellow)),
            Span::raw("  Set batch window (milliseconds)"),
        ]),
        Line::from(vec![
            Span::styled("  :bw fast/medium/slow", Style::default().fg(Color::Yellow)),
            Span::raw("  Presets: 100ms/1000ms/5000ms"),
        ]),
        Line::from(vec![
            Span::styled("  +/-", Style::default().fg(Color::Yellow)),
            Span::raw("     Increase/decrease batch window by 100ms"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Clipboard & Batch:", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  c", Style::default().fg(Color::Yellow)),
            Span::raw("       Copy selected line to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  C", Style::default().fg(Color::Yellow)),
            Span::raw("       Copy entire batch to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  b", Style::default().fg(Color::Yellow)),
            Span::raw("       Focus on batch containing selected line"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::styled(" or ", Style::default()),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled(" to close this help", Style::default()),
        ]),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });

    let area = centered_rect(60, 80, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

/// Draw the expanded line view overlay
fn draw_expanded_line_overlay(f: &mut Frame, manager: &ProcessManager, app: &App) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Modifier, Style};

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
        Span::styled(" or ", Style::default()),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" to close", Style::default()),
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
