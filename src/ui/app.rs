use std::collections::HashMap;

use chrono::{DateTime, Local};

use super::batch_state::BatchState;
use super::click_regions::ClickRegions;
use super::display_state::DisplayState;
use super::filter_state::FilterState;
use super::input_state::InputState;
use super::navigation_state::NavigationState;
use super::process_colors::ProcessColors;
use super::render_cache::RenderCache;
use super::trace_state::TraceState;
use crate::log::LogLine;
use crate::traces::TraceCandidate;

/// Display mode for log lines
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    /// Condense [key:value] metadata, truncate long lines
    #[default]
    Compact,
    /// Show all content, truncate long lines
    Full,
    /// Show all content, wrap long lines
    Wrap,
}

impl DisplayMode {
    /// Cycle to the next display mode
    pub fn next(self) -> Self {
        match self {
            DisplayMode::Compact => DisplayMode::Full,
            DisplayMode::Full => DisplayMode::Wrap,
            DisplayMode::Wrap => DisplayMode::Compact,
        }
    }

    /// Get a human-readable name for the mode
    pub fn name(self) -> &'static str {
        match self {
            DisplayMode::Compact => "compact",
            DisplayMode::Full => "full",
            DisplayMode::Wrap => "wrap",
        }
    }
}

/// Application state for the TUI
pub struct App {
    /// Input and command state
    pub input: InputState,
    /// Navigation and scroll state
    pub navigation: NavigationState,
    /// Filter state for log filtering
    pub filters: FilterState,
    /// Batch mode state
    pub batch: BatchState,
    /// Trace mode state
    pub trace: TraceState,
    /// Display state for UI modes
    pub display: DisplayState,
    /// Render caches for performance
    pub cache: RenderCache,
    /// Mouse click regions
    pub regions: ClickRegions,
    /// Colors assigned to each process/log file
    pub process_colors: ProcessColors,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Whether we're in the process of shutting down
    pub shutting_down: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            navigation: NavigationState::new(),
            filters: FilterState::new(),
            batch: BatchState::new(),
            trace: TraceState::new(),
            display: DisplayState::new(),
            cache: RenderCache::new(),
            regions: ClickRegions::new(),
            process_colors: ProcessColors::new(&[], &[], &HashMap::new()),
            should_quit: false,
            shutting_down: false,
        }
    }

    /// Initialize process colors from config and process/log file names.
    pub fn init_process_colors(
        &mut self,
        process_names: &[String],
        log_file_names: &[String],
        config_colors: &HashMap<String, String>,
    ) {
        self.process_colors = ProcessColors::new(process_names, log_file_names, config_colors);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn start_shutdown(&mut self) {
        self.shutting_down = true;
        // Clear any active modes
        self.input.command_mode = false;
        self.input.search_mode = false;
        self.display.show_help = false;
        self.display.expanded_line_view = false;
        self.set_status_info("Shutting down processes...".to_string());
    }

    // Delegation methods for backward compatibility during migration
    // These will be removed in Phase 3 as call sites update to use sub-structs directly

    pub fn scroll_up(&mut self, lines: usize) {
        self.navigation.scroll_up(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, max_offset: usize) {
        self.navigation.scroll_down(lines, max_offset);
    }

    pub fn scroll_to_top(&mut self) {
        self.navigation.scroll_to_top();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.navigation.scroll_to_bottom();
    }

    pub fn enter_command_mode(&mut self) {
        self.input.enter_command_mode();
        self.display.status_message = None;
    }

    pub fn exit_command_mode(&mut self) {
        self.input.exit_command_mode();
    }

    pub fn add_char(&mut self, c: char) {
        self.input.add_char(c);
    }

    pub fn delete_char(&mut self) {
        self.input.delete_char();
    }

    pub fn set_status_success(&mut self, message: String) {
        self.display.set_status_success(message);
    }

    pub fn set_status_error(&mut self, message: String) {
        self.display.set_status_error(message);
    }

    pub fn set_status_info(&mut self, message: String) {
        self.display.set_status_info(message);
    }

    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.display.clear_status();
    }

    pub fn save_to_history(&mut self, command: String) {
        self.input.save_to_history(command);
    }

    pub fn history_prev(&mut self) {
        self.input.history_prev();
    }

    pub fn history_next(&mut self) {
        self.input.history_next();
    }

    pub fn reset_history_nav(&mut self) {
        self.input.reset_history_nav();
    }

    pub fn add_include_filter(&mut self, pattern: String) {
        self.filters.add_include_filter(pattern);
    }

    pub fn add_exclude_filter(&mut self, pattern: String) {
        self.filters.add_exclude_filter(pattern);
    }

    pub fn clear_filters(&mut self) {
        self.filters.clear_filters();
    }

    pub fn remove_filter(&mut self, pattern: &str) -> bool {
        self.filters.remove_filter(pattern)
    }

    pub fn filter_count(&self) -> usize {
        self.filters.filter_count()
    }

    pub fn enter_search_mode(&mut self) {
        self.input.enter_search_mode();
    }

    pub fn exit_search_mode(&mut self) {
        self.input.exit_search_mode();
    }

    pub fn perform_search(&mut self, pattern: String) {
        self.input.perform_search(pattern);
        // Close expanded view when a new search is performed
        self.display.expanded_line_view = false;
    }

    pub fn clear_search(&mut self) {
        self.input.clear_search();
    }

    pub fn next_batch(&mut self) {
        self.batch.next_batch();
        self.navigation.scroll_offset = 0;
        self.navigation.auto_scroll = false;
    }

    pub fn prev_batch(&mut self) {
        self.batch.prev_batch();
        self.navigation.scroll_offset = 0;
        self.navigation.auto_scroll = false;
    }

    pub fn toggle_batch_view(&mut self) {
        self.batch.toggle_batch_view();
    }

    pub fn set_batch_window(&mut self, window_ms: i64) {
        self.batch.set_batch_window(window_ms);
        if self.batch.batch_view_mode {
            self.navigation.scroll_offset = 0;
        }
    }

    pub fn toggle_help(&mut self) {
        self.display.toggle_help();
    }

    pub fn scroll_help_up(&mut self) {
        self.display.scroll_help_up();
    }

    pub fn scroll_help_down(&mut self) {
        self.display.scroll_help_down();
    }

    #[allow(dead_code)]
    pub fn select_line_by_id(&mut self, id: Option<u64>) {
        self.navigation.select_line_by_id(id);
    }

    #[allow(dead_code)]
    pub fn clear_selection(&mut self) {
        self.navigation.clear_selection();
    }

    pub fn toggle_expanded_view(&mut self) {
        self.display.toggle_expanded_view();
    }

    pub fn close_expanded_view(&mut self) {
        self.display.close_expanded_view();
    }

    pub fn freeze_display(&mut self) {
        self.navigation.freeze_display();
    }

    pub fn unfreeze_display(&mut self) {
        self.navigation.unfreeze_display();
    }

    pub fn create_snapshot(&mut self, logs: Vec<LogLine>) {
        self.navigation.create_snapshot(logs);
    }

    pub fn discard_snapshot(&mut self) {
        self.navigation.discard_snapshot();
    }

    /// Enter trace selection mode with a list of candidates
    pub fn enter_trace_selection(&mut self, candidates: Vec<TraceCandidate>) {
        self.trace.enter_trace_selection(candidates);
    }

    pub fn select_next_trace(&mut self) {
        self.trace.select_next_trace();
    }

    pub fn select_prev_trace(&mut self) {
        self.trace.select_prev_trace();
    }

    pub fn exit_trace_selection(&mut self) {
        self.trace.exit_trace_selection();
    }

    pub fn get_selected_trace(&self) -> Option<&TraceCandidate> {
        self.trace.get_selected_trace()
    }

    /// Enter trace filter mode for a specific trace
    pub fn enter_trace_filter(&mut self, trace_id: String, start: DateTime<Local>, end: DateTime<Local>) {
        self.trace.enter_trace_filter(trace_id, start, end);
        self.navigation.freeze_display();
    }

    pub fn expand_trace_before(&mut self) {
        self.trace.expand_trace_before();
    }

    pub fn expand_trace_after(&mut self) {
        self.trace.expand_trace_after();
    }

    pub fn exit_trace_filter(&mut self) {
        self.trace.exit_trace_filter();
        self.navigation.unfreeze_display();
        self.navigation.selected_line_id = None;
    }

    pub fn cycle_display_mode(&mut self) {
        self.display.cycle_display_mode();
    }

    pub fn is_compact(&self) -> bool {
        self.display.is_compact()
    }

    pub fn is_wrap(&self) -> bool {
        self.display.is_wrap()
    }
}
