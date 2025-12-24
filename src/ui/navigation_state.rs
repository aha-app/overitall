use chrono::{DateTime, Local};

use crate::log::LogLine;

/// Navigation and scroll state for the log viewer
#[derive(Debug, Default)]
pub struct NavigationState {
    /// Scroll offset for log viewer (lines from top)
    pub scroll_offset: usize,
    /// Whether to auto-scroll to bottom (stick to latest logs)
    pub auto_scroll: bool,
    /// ID of selected line (for line expansion/clipboard)
    pub selected_line_id: Option<u64>,
    /// Whether log display is frozen (paused during selection/review)
    pub frozen: bool,
    /// Timestamp when display was frozen (used to filter out newer logs)
    pub frozen_at: Option<DateTime<Local>>,
    /// Snapshot of logs when entering frozen/batch mode (immune to eviction)
    pub snapshot: Option<Vec<LogLine>>,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            auto_scroll: true,
            ..Default::default()
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, lines: usize, max_offset: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
        if self.scroll_offset >= max_offset {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll_offset = 0;
    }

    pub fn freeze_display(&mut self) {
        if !self.frozen {
            self.frozen = true;
            self.frozen_at = Some(Local::now());
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

    #[allow(dead_code)]
    pub fn select_line_by_id(&mut self, id: Option<u64>) {
        self.selected_line_id = id;
    }

    #[allow(dead_code)]
    pub fn clear_selection(&mut self) {
        self.selected_line_id = None;
    }
}
