use std::collections::HashSet;

use chrono::{DateTime, Duration, Local};

use crate::log::LogLine;
use crate::traces::TraceCandidate;
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
    /// ID of selected line (for line expansion/clipboard)
    pub selected_line_id: Option<u64>,
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

    // Trace selection mode (picking from list of detected traces)
    /// Whether we're in trace selection mode
    pub trace_selection_mode: bool,
    /// Detected trace candidates to choose from
    pub trace_candidates: Vec<TraceCandidate>,
    /// Currently selected trace index in the list
    pub selected_trace_index: usize,

    // Trace filter mode (viewing a specific trace)
    /// Whether we're in trace filter mode (focused on a single trace)
    pub trace_filter_mode: bool,
    /// The active trace ID being filtered
    pub active_trace_id: Option<String>,
    /// Start time of the trace (first occurrence)
    pub trace_time_start: Option<DateTime<Local>>,
    /// End time of the trace (last occurrence)
    pub trace_time_end: Option<DateTime<Local>>,
    /// How much to expand view before the first trace occurrence
    pub trace_expand_before: Duration,
    /// How much to expand view after the last trace occurrence
    pub trace_expand_after: Duration,

    // Manual trace capture
    /// Whether we're currently recording a manual trace
    pub manual_trace_recording: bool,
    /// When manual trace recording started
    pub manual_trace_start: Option<DateTime<Local>>,

    // Help overlay
    /// Scroll offset for help overlay
    pub help_scroll_offset: u16,
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
            selected_line_id: None,
            expanded_line_view: false,
            shutting_down: false,
            frozen: false,
            frozen_at: None,
            snapshot: None,
            hidden_processes: HashSet::new(),

            // Trace selection mode
            trace_selection_mode: false,
            trace_candidates: Vec::new(),
            selected_trace_index: 0,

            // Trace filter mode
            trace_filter_mode: false,
            active_trace_id: None,
            trace_time_start: None,
            trace_time_end: None,
            trace_expand_before: Duration::zero(),
            trace_expand_after: Duration::zero(),

            // Manual trace capture
            manual_trace_recording: false,
            manual_trace_start: None,

            // Help overlay
            help_scroll_offset: 0,
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
        if self.show_help {
            self.help_scroll_offset = 0; // Reset scroll when opening
        }
    }

    pub fn scroll_help_up(&mut self) {
        self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
    }

    pub fn scroll_help_down(&mut self) {
        self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
    }

    pub fn select_line_by_id(&mut self, id: Option<u64>) {
        self.selected_line_id = id;
    }

    pub fn clear_selection(&mut self) {
        self.selected_line_id = None;
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

    // Trace selection mode methods

    /// Enter trace selection mode with a list of candidates
    pub fn enter_trace_selection(&mut self, candidates: Vec<TraceCandidate>) {
        self.trace_selection_mode = true;
        self.trace_candidates = candidates;
        self.selected_trace_index = 0;
    }

    /// Move selection to next trace candidate
    pub fn select_next_trace(&mut self) {
        if !self.trace_candidates.is_empty() {
            self.selected_trace_index = (self.selected_trace_index + 1) % self.trace_candidates.len();
        }
    }

    /// Move selection to previous trace candidate
    pub fn select_prev_trace(&mut self) {
        if !self.trace_candidates.is_empty() {
            if self.selected_trace_index == 0 {
                self.selected_trace_index = self.trace_candidates.len() - 1;
            } else {
                self.selected_trace_index -= 1;
            }
        }
    }

    /// Exit trace selection mode without selecting
    pub fn exit_trace_selection(&mut self) {
        self.trace_selection_mode = false;
        self.trace_candidates.clear();
        self.selected_trace_index = 0;
    }

    /// Get the currently selected trace candidate
    pub fn get_selected_trace(&self) -> Option<&TraceCandidate> {
        self.trace_candidates.get(self.selected_trace_index)
    }

    // Trace filter mode methods

    /// Enter trace filter mode for a specific trace
    pub fn enter_trace_filter(&mut self, trace_id: String, start: DateTime<Local>, end: DateTime<Local>) {
        self.trace_filter_mode = true;
        self.active_trace_id = Some(trace_id);
        self.trace_time_start = Some(start);
        self.trace_time_end = Some(end);
        self.trace_expand_before = Duration::zero();
        self.trace_expand_after = Duration::zero();
        self.freeze_display();
    }

    /// Expand trace view backward (show more context before trace)
    pub fn expand_trace_before(&mut self) {
        self.trace_expand_before = self.trace_expand_before + Duration::seconds(5);
    }

    /// Expand trace view forward (show more context after trace)
    pub fn expand_trace_after(&mut self) {
        self.trace_expand_after = self.trace_expand_after + Duration::seconds(5);
    }

    /// Exit trace filter mode
    pub fn exit_trace_filter(&mut self) {
        self.trace_filter_mode = false;
        self.active_trace_id = None;
        self.trace_time_start = None;
        self.trace_time_end = None;
        self.trace_expand_before = Duration::zero();
        self.trace_expand_after = Duration::zero();
        self.unfreeze_display();
        self.selected_line_id = None;
    }
}
