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
    /// Line ID where multi-select started (Shift+arrow selection)
    pub selection_anchor: Option<u64>,
    /// Current end of multi-select range
    pub selection_end: Option<u64>,
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

    /// Check if a line ID is within the multi-select range
    pub fn is_in_selection(&self, id: u64, display_logs: &[LogLine]) -> bool {
        let (anchor, end) = match (self.selection_anchor, self.selection_end) {
            (Some(a), Some(e)) => (a, e),
            _ => return false,
        };

        // Find positions of anchor and end in display order
        let mut anchor_pos = None;
        let mut end_pos = None;
        let mut target_pos = None;

        for (i, log) in display_logs.iter().enumerate() {
            if log.id == anchor {
                anchor_pos = Some(i);
            }
            if log.id == end {
                end_pos = Some(i);
            }
            if log.id == id {
                target_pos = Some(i);
            }
        }

        match (anchor_pos, end_pos, target_pos) {
            (Some(a), Some(e), Some(t)) => {
                let (start, finish) = if a <= e { (a, e) } else { (e, a) };
                t >= start && t <= finish
            }
            _ => false,
        }
    }

    /// Check if a line ID is within the multi-select range (for reference slices)
    pub fn is_in_selection_ref(&self, id: u64, display_logs: &[&LogLine]) -> bool {
        let (anchor, end) = match (self.selection_anchor, self.selection_end) {
            (Some(a), Some(e)) => (a, e),
            _ => return false,
        };

        let mut anchor_pos = None;
        let mut end_pos = None;
        let mut target_pos = None;

        for (i, log) in display_logs.iter().enumerate() {
            if log.id == anchor {
                anchor_pos = Some(i);
            }
            if log.id == end {
                end_pos = Some(i);
            }
            if log.id == id {
                target_pos = Some(i);
            }
        }

        match (anchor_pos, end_pos, target_pos) {
            (Some(a), Some(e), Some(t)) => {
                let (start, finish) = if a <= e { (a, e) } else { (e, a) };
                t >= start && t <= finish
            }
            _ => false,
        }
    }

    /// Check if multi-select is active
    pub fn has_multi_select(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_end.is_some()
    }

    /// Clear multi-select state
    pub fn clear_multi_select(&mut self) {
        self.selection_anchor = None;
        self.selection_end = None;
    }

    /// Start or continue multi-select from current position
    pub fn start_multi_select(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = self.selected_line_id;
        }
    }

    /// Update the end of multi-select range
    pub fn set_selection_end(&mut self, id: u64) {
        self.selection_end = Some(id);
        self.selected_line_id = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;

    fn make_logs(ids: &[u64]) -> Vec<LogLine> {
        ids.iter()
            .map(|id| {
                let mut log = LogLine::new(LogSource::ProcessStdout("test".to_string()), "".to_string());
                // Override the auto-assigned ID
                log.id = *id;
                log
            })
            .collect()
    }

    #[test]
    fn test_is_in_selection_no_anchor() {
        let nav = NavigationState::new();
        let logs = make_logs(&[1, 2, 3]);
        assert!(!nav.is_in_selection(2, &logs));
    }

    #[test]
    fn test_is_in_selection_forward_range() {
        let mut nav = NavigationState::new();
        nav.selection_anchor = Some(1);
        nav.selection_end = Some(3);
        let logs = make_logs(&[1, 2, 3, 4, 5]);

        assert!(nav.is_in_selection(1, &logs)); // anchor
        assert!(nav.is_in_selection(2, &logs)); // middle
        assert!(nav.is_in_selection(3, &logs)); // end
        assert!(!nav.is_in_selection(4, &logs)); // outside
        assert!(!nav.is_in_selection(5, &logs)); // outside
    }

    #[test]
    fn test_is_in_selection_backward_range() {
        let mut nav = NavigationState::new();
        nav.selection_anchor = Some(4);
        nav.selection_end = Some(2);
        let logs = make_logs(&[1, 2, 3, 4, 5]);

        assert!(!nav.is_in_selection(1, &logs)); // outside
        assert!(nav.is_in_selection(2, &logs)); // end
        assert!(nav.is_in_selection(3, &logs)); // middle
        assert!(nav.is_in_selection(4, &logs)); // anchor
        assert!(!nav.is_in_selection(5, &logs)); // outside
    }

    #[test]
    fn test_is_in_selection_single_line() {
        let mut nav = NavigationState::new();
        nav.selection_anchor = Some(2);
        nav.selection_end = Some(2);
        let logs = make_logs(&[1, 2, 3]);

        assert!(!nav.is_in_selection(1, &logs));
        assert!(nav.is_in_selection(2, &logs));
        assert!(!nav.is_in_selection(3, &logs));
    }

    #[test]
    fn test_has_multi_select() {
        let mut nav = NavigationState::new();
        assert!(!nav.has_multi_select());

        nav.selection_anchor = Some(1);
        assert!(!nav.has_multi_select());

        nav.selection_end = Some(2);
        assert!(nav.has_multi_select());
    }

    #[test]
    fn test_clear_multi_select() {
        let mut nav = NavigationState::new();
        nav.selection_anchor = Some(1);
        nav.selection_end = Some(3);

        nav.clear_multi_select();

        assert!(nav.selection_anchor.is_none());
        assert!(nav.selection_end.is_none());
    }

    #[test]
    fn test_start_multi_select() {
        let mut nav = NavigationState::new();
        nav.selected_line_id = Some(5);

        nav.start_multi_select();

        assert_eq!(nav.selection_anchor, Some(5));

        // Second call should not change anchor
        nav.selected_line_id = Some(10);
        nav.start_multi_select();
        assert_eq!(nav.selection_anchor, Some(5));
    }

    #[test]
    fn test_set_selection_end() {
        let mut nav = NavigationState::new();
        nav.selection_anchor = Some(1);

        nav.set_selection_end(5);

        assert_eq!(nav.selection_end, Some(5));
        assert_eq!(nav.selected_line_id, Some(5)); // Also updates cursor
    }
}
