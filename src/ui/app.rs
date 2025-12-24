use std::collections::HashMap;

use super::batch_state::BatchState;
use super::click_regions::ClickRegions;
use super::display_state::DisplayState;
use super::filter_state::FilterState;
use super::input_state::InputState;
use super::navigation_state::NavigationState;
use super::process_colors::ProcessColors;
use super::render_cache::RenderCache;
use super::trace_state::TraceState;

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
}
