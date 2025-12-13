use std::collections::HashSet;

use crate::log::LogLine;
use super::filter::{Filter, FilterType};
use super::types::StatusType;

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
    /// Whether we're in the process of shutting down
    pub shutting_down: bool,
    /// Whether log display is frozen (paused during selection/review)
    pub frozen: bool,
    /// Timestamp when display was frozen (used to filter out newer logs)
    pub frozen_at: Option<chrono::DateTime<chrono::Local>>,
    /// Snapshot of logs when entering frozen/batch mode (immune to eviction)
    pub snapshot: Option<Vec<LogLine>>,
    /// Set of process names whose output should be hidden
    pub hidden_processes: HashSet<String>,
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
            batch_window_ms: 100,
            batch_view_mode: false,
            current_batch: None,
            show_help: false,
            selected_line_index: None,
            expanded_line_view: false,
            shutting_down: false,
            frozen: false,
            frozen_at: None,
            snapshot: None,
            hidden_processes: HashSet::new(),
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn start_shutdown(&mut self) {
        self.shutting_down = true;
        // Clear any active modes
        self.command_mode = false;
        self.search_mode = false;
        self.show_help = false;
        self.expanded_line_view = false;
        self.set_status_info("Shutting down processes...".to_string());
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
        self.search_pattern.clear();
    }

    pub fn perform_search(&mut self, pattern: String) {
        self.search_pattern = pattern;
    }

    pub fn clear_search(&mut self) {
        self.search_pattern.clear();
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
        let was_none = self.selected_line_index.is_none();
        self.selected_line_index = Some(match self.selected_line_index {
            None => 0,
            Some(idx) if idx >= max_lines - 1 => 0, // Wrap to top when at bottom
            Some(idx) => idx + 1,
        });
        self.auto_scroll = false;
        if was_none {
            self.freeze_display();
        }
    }

    pub fn select_prev_line(&mut self, max_lines: usize) {
        if max_lines == 0 {
            return;
        }
        let was_none = self.selected_line_index.is_none();
        self.selected_line_index = Some(match self.selected_line_index {
            None => max_lines - 1, // When tailing, Up arrow selects the last (most recent) line
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => max_lines - 1, // Wrap to bottom when at top
        });
        self.auto_scroll = false;
        if was_none {
            self.freeze_display();
        }
    }

    pub fn toggle_expanded_view(&mut self) {
        self.expanded_line_view = !self.expanded_line_view;
    }

    pub fn close_expanded_view(&mut self) {
        self.expanded_line_view = false;
    }

    pub fn freeze_display(&mut self) {
        if !self.frozen {
            self.frozen = true;
            self.frozen_at = Some(chrono::Local::now());
        }
    }

    pub fn unfreeze_display(&mut self) {
        self.frozen = false;
        self.frozen_at = None;
    }

    pub fn create_snapshot(&mut self, logs: Vec<LogLine>) {
        self.snapshot = Some(logs);
    }

    pub fn discard_snapshot(&mut self) {
        self.snapshot = None;
    }
}
