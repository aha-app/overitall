use std::collections::HashSet;

use super::filter::{Filter, FilterType};

/// Filter state for log filtering
#[derive(Debug, Default)]
pub struct FilterState {
    /// Active log filters
    pub filters: Vec<Filter>,
    /// Set of process names whose output should be hidden
    pub hidden_processes: HashSet<String>,
}

impl FilterState {
    pub fn new() -> Self {
        Self::default()
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

    pub fn remove_filter(&mut self, pattern: &str) -> bool {
        let original_len = self.filters.len();
        self.filters.retain(|f| f.pattern != pattern);
        self.filters.len() < original_len
    }

    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }
}
