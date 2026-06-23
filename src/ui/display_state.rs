use super::app::DisplayMode;
use super::types::StatusType;

/// Process panel view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProcessPanelViewMode {
    /// All processes with grid layout and status dots
    #[default]
    Normal,
    /// Process count + only non-running processes
    Summary,
    /// Just the process count
    Minimal,
}

impl ProcessPanelViewMode {
    /// Cycle to the next view mode
    pub fn next(self) -> Self {
        match self {
            ProcessPanelViewMode::Normal => ProcessPanelViewMode::Summary,
            ProcessPanelViewMode::Summary => ProcessPanelViewMode::Minimal,
            ProcessPanelViewMode::Minimal => ProcessPanelViewMode::Normal,
        }
    }

    /// Get a human-readable name for the mode
    pub fn name(self) -> &'static str {
        match self {
            ProcessPanelViewMode::Normal => "normal",
            ProcessPanelViewMode::Summary => "summary",
            ProcessPanelViewMode::Minimal => "minimal",
        }
    }
}

/// What the main content area renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentView {
    /// Log viewer (default).
    #[default]
    Logs,
    /// Tree of managed processes and their descendants.
    ProcessTree,
}

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
    /// Current process panel view mode
    pub process_panel_mode: ProcessPanelViewMode,
    /// What the main content area renders (logs or process tree)
    pub content_view: ContentView,
    /// Scroll offset (in lines) for the process tree viewer
    pub process_tree_scroll: u16,
    /// Last rendered viewport height of the process tree (set by the widget)
    pub process_tree_viewport: u16,
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
            process_panel_mode: ProcessPanelViewMode::Normal,
            content_view: ContentView::Logs,
            process_tree_scroll: 0,
            process_tree_viewport: 0,
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

    pub fn cycle_process_panel_mode(&mut self) {
        self.process_panel_mode = self.process_panel_mode.next();
    }

    /// Toggle the content area between logs and the process tree viewer.
    /// Returns true if the process tree is now showing.
    pub fn toggle_process_tree(&mut self) -> bool {
        self.content_view = match self.content_view {
            ContentView::Logs => ContentView::ProcessTree,
            ContentView::ProcessTree => ContentView::Logs,
        };
        // Reset scroll when entering the tree so it always opens at the top.
        if self.content_view == ContentView::ProcessTree {
            self.process_tree_scroll = 0;
        }
        self.content_view == ContentView::ProcessTree
    }

    pub fn show_logs(&mut self) {
        self.content_view = ContentView::Logs;
    }

    pub fn is_process_tree(&self) -> bool {
        self.content_view == ContentView::ProcessTree
    }

    /// Scroll the process tree up by `n` lines.
    pub fn process_tree_scroll_up(&mut self, n: u16) {
        self.process_tree_scroll = self.process_tree_scroll.saturating_sub(n);
    }

    /// Scroll the process tree down by `n` lines. The widget clamps the offset
    /// to the rendered content on the next draw.
    pub fn process_tree_scroll_down(&mut self, n: u16) {
        self.process_tree_scroll = self.process_tree_scroll.saturating_add(n);
    }

    /// Scroll the process tree to the top.
    pub fn process_tree_scroll_home(&mut self) {
        self.process_tree_scroll = 0;
    }

    /// Scroll the process tree to the bottom. The widget clamps the offset to
    /// the last page on the next draw.
    pub fn process_tree_scroll_end(&mut self) {
        self.process_tree_scroll = u16::MAX;
    }

    /// Page size for the process tree, based on the last rendered viewport.
    pub fn process_tree_page(&self) -> u16 {
        self.process_tree_viewport.max(1)
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
