use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

    /// Check if a log line matches this filter
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
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.auto_scroll = false;
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, lines: usize, max_offset: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
        // If we scrolled to the bottom, re-enable auto-scroll
        if self.scroll_offset >= max_offset {
            self.auto_scroll = true;
        }
    }

    /// Jump to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    /// Jump to bottom and enable auto-scroll
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll_offset = 0; // Will be recalculated when auto_scroll is true
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.command_mode = true;
        self.input.clear();
        self.status_message = None; // Clear status when entering command mode
        self.history_index = None; // Reset history navigation
    }

    /// Exit command mode
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

    /// Set a success status message
    pub fn set_status_success(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Success));
    }

    /// Set an error status message
    pub fn set_status_error(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Error));
    }

    /// Set an info status message
    pub fn set_status_info(&mut self, message: String) {
        self.status_message = Some((message, StatusType::Info));
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Save a command to history
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

    /// Add an include filter
    pub fn add_include_filter(&mut self, pattern: String) {
        self.filters.push(Filter::new(pattern, FilterType::Include));
    }

    /// Add an exclude filter
    pub fn add_exclude_filter(&mut self, pattern: String) {
        self.filters.push(Filter::new(pattern, FilterType::Exclude));
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    /// Get count of active filters
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    /// Enter search mode
    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.input.clear();
    }

    /// Exit search mode
    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.input.clear();
    }

    /// Perform a search
    pub fn perform_search(&mut self, pattern: String) {
        self.search_pattern = pattern;
        self.current_match = Some(0);
    }

    /// Clear the search
    pub fn clear_search(&mut self) {
        self.search_pattern.clear();
        self.current_match = None;
    }

    /// Move to next search match
    pub fn next_match(&mut self, total_matches: usize) {
        if total_matches == 0 {
            return;
        }
        if let Some(idx) = self.current_match {
            self.current_match = Some((idx + 1) % total_matches);
        }
    }

    /// Move to previous search match
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
}

/// Draw the UI to the terminal
pub fn draw(f: &mut Frame, app: &App, manager: &ProcessManager) {
    // Create the main layout: process list, log viewer, command input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10), // Process list
            Constraint::Percentage(85), // Log viewer
            Constraint::Percentage(5),  // Command input
        ])
        .split(f.area());

    // Draw process list
    draw_process_list(f, chunks[0], manager);

    // Draw log viewer
    draw_log_viewer(f, chunks[1], manager, app);

    // Draw command input
    draw_command_input(f, chunks[2], app);
}

/// Draw the process list at the top of the screen
fn draw_process_list(f: &mut Frame, area: ratatui::layout::Rect, manager: &ProcessManager) {
    let processes = manager.get_all_statuses();

    let items: Vec<ListItem> = processes
        .iter()
        .map(|(name, status)| {
            let (status_text, color) = match status {
                ProcessStatus::Running => ("Running", Color::Green),
                ProcessStatus::Stopped => ("Stopped", Color::Red),
                ProcessStatus::Failed(_) => ("Failed", Color::Red),
            };

            let content = Line::from(vec![
                Span::raw("• "),
                Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" ["),
                Span::styled(status_text, Style::default().fg(color)),
                Span::raw("]"),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Processes ")
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    f.render_widget(list, area);
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

    // Calculate visible lines (area height minus 2 for borders)
    let visible_lines = area.height.saturating_sub(2) as usize;
    let total_logs = filtered_logs.len();

    // Find search matches
    let search_matches: Vec<usize> = if !app.search_pattern.is_empty() {
        filtered_logs
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
        let display = &filtered_logs[start..];
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
            let display = &filtered_logs[start..end];
            (display, String::new(), start)
        } else {
            // Invalid match index, fall back to manual scroll
            let start = app.scroll_offset.min(total_logs.saturating_sub(visible_lines));
            let end = (start + visible_lines).min(total_logs);
            let display = &filtered_logs[start..end];
            (display, String::new(), start)
        }
    } else {
        // Manual scroll mode: show logs from scroll_offset
        let start = app.scroll_offset.min(total_logs.saturating_sub(visible_lines));
        let end = (start + visible_lines).min(total_logs);
        let display = &filtered_logs[start..end];

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
    let log_lines: Vec<Line> = display_logs
        .iter()
        .enumerate()
        .map(|(display_idx, log)| {
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

            // Choose style based on whether this is a match
            let line_style = if is_current_match {
                // Current match: yellow background
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else if is_match {
                // Other matches: dark gray background
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Line::from(vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{}: ", process_name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&log.line, line_style),
            ])
        })
        .collect();

    // Build title with filters and search info
    let mut title_parts = vec![" Logs ".to_string()];

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

    let paragraph = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

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
