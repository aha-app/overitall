/// Batch mode state for grouping log lines by time window
#[derive(Debug)]
pub struct BatchState {
    /// Time window for batch detection in milliseconds
    pub batch_window_ms: i64,
    /// If true, show only the current batch
    pub batch_view_mode: bool,
    /// Index of currently viewed batch
    pub current_batch: Option<usize>,
}

impl Default for BatchState {
    fn default() -> Self {
        Self {
            batch_window_ms: 100,
            batch_view_mode: false,
            current_batch: None,
        }
    }
}

impl BatchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_batch(&mut self) {
        if let Some(current) = self.current_batch {
            self.current_batch = Some(current + 1);
        } else {
            self.current_batch = Some(0);
        }
        self.batch_view_mode = true;
    }

    pub fn prev_batch(&mut self) {
        if let Some(current) = self.current_batch {
            if current > 0 {
                self.current_batch = Some(current - 1);
            }
        }
        self.batch_view_mode = true;
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
        if self.batch_view_mode {
            self.current_batch = Some(0);
        }
    }

    pub fn reset_scroll_state(&self) -> (usize, bool) {
        (0, false)
    }
}
