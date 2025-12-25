use super::app::DisplayMode;
use super::types::StatusType;

/// Timestamp display mode for log lines
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimestampMode {
    /// Show time as HH:MM:SS (default)
    #[default]
    Seconds,
    /// Show time as HH:MM:SS.mmm (with milliseconds)
    Milliseconds,
    /// Hide timestamps entirely
    Off,
}

impl TimestampMode {
    /// Cycle to the next timestamp mode
    pub fn next(self) -> Self {
        match self {
            TimestampMode::Seconds => TimestampMode::Milliseconds,
            TimestampMode::Milliseconds => TimestampMode::Off,
            TimestampMode::Off => TimestampMode::Seconds,
        }
    }

    /// Get a human-readable name for the mode
    pub fn name(self) -> &'static str {
        match self {
            TimestampMode::Seconds => "seconds",
            TimestampMode::Milliseconds => "milliseconds",
            TimestampMode::Off => "off",
        }
    }
}

/// Display state for UI modes and status
#[derive(Debug)]
pub struct DisplayState {
    /// Current display mode (compact, full, or wrap)
    pub display_mode: DisplayMode,
    /// Current timestamp display mode
    pub timestamp_mode: TimestampMode,
    /// Whether to show the help overlay
    pub show_help: bool,
    /// Scroll offset for help overlay
    pub help_scroll_offset: u16,
    /// Whether to show expanded line view
    pub expanded_line_view: bool,
    /// Status message to show to the user (message, type)
    pub status_message: Option<(String, StatusType)>,
    /// Whether process coloring is enabled
    pub coloring_enabled: bool,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Compact,
            timestamp_mode: TimestampMode::Seconds,
            show_help: false,
            help_scroll_offset: 0,
            expanded_line_view: false,
            status_message: None,
            coloring_enabled: false,
        }
    }
}

impl DisplayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cycle_display_mode(&mut self) {
        self.display_mode = self.display_mode.next();
    }

    pub fn is_compact(&self) -> bool {
        self.display_mode == DisplayMode::Compact
    }

    pub fn is_wrap(&self) -> bool {
        self.display_mode == DisplayMode::Wrap
    }

    pub fn cycle_timestamp_mode(&mut self) {
        self.timestamp_mode = self.timestamp_mode.next();
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_scroll_offset = 0;
        }
    }

    pub fn scroll_help_up(&mut self) {
        self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
    }

    pub fn scroll_help_down(&mut self) {
        self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
    }

    pub fn toggle_expanded_view(&mut self) {
        self.expanded_line_view = !self.expanded_line_view;
    }

    pub fn close_expanded_view(&mut self) {
        self.expanded_line_view = false;
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

    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }
}
