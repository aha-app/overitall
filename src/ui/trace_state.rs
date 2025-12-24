use chrono::{DateTime, Duration, Local};

use crate::traces::TraceCandidate;

/// Trace mode state for trace selection and filtering
#[derive(Debug, Default)]
pub struct TraceState {
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
}

impl TraceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enter trace selection mode with a list of candidates
    pub fn enter_trace_selection(&mut self, candidates: Vec<TraceCandidate>) {
        self.trace_selection_mode = true;
        self.trace_candidates = candidates;
        self.selected_trace_index = 0;
    }

    /// Move selection to next trace candidate
    pub fn select_next_trace(&mut self) {
        if !self.trace_candidates.is_empty() {
            self.selected_trace_index =
                (self.selected_trace_index + 1) % self.trace_candidates.len();
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

    /// Enter trace filter mode for a specific trace
    pub fn enter_trace_filter(&mut self, trace_id: String, start: DateTime<Local>, end: DateTime<Local>) {
        self.trace_filter_mode = true;
        self.active_trace_id = Some(trace_id);
        self.trace_time_start = Some(start);
        self.trace_time_end = Some(end);
        self.trace_expand_before = Duration::zero();
        self.trace_expand_after = Duration::zero();
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
    }
}
