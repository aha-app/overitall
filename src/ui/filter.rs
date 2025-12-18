use crate::log::LogLine;

/// Filter type
#[derive(Debug, Clone)]
pub enum FilterType {
    Include,
    Exclude,
}

/// A log filter
#[derive(Debug, Clone)]
pub struct Filter {
    pub pattern: String,
    /// Pre-computed lowercase pattern for case-insensitive matching (avoids allocation per match)
    pattern_lowercase: String,
    pub filter_type: FilterType,
    pub is_regex: bool, // For future: support both plain text and regex
}

impl Filter {
    /// Create a new filter (start with plain text matching)
    pub fn new(pattern: String, filter_type: FilterType) -> Self {
        let pattern_lowercase = pattern.to_lowercase();
        Self {
            pattern,
            pattern_lowercase,
            filter_type,
            is_regex: false, // Start with plain text, add regex support later
        }
    }

    /// Check if a line matches the filter (calls to_lowercase() on line)
    /// For better performance, use matches_lowercase() with pre-computed lowercase
    pub fn matches(&self, line: &str) -> bool {
        self.matches_lowercase(&line.to_lowercase())
    }

    /// Check if a pre-computed lowercase line matches the filter (no allocation)
    pub fn matches_lowercase(&self, line_lowercase: &str) -> bool {
        if self.is_regex {
            // Future: regex matching
            false
        } else {
            line_lowercase.contains(&self.pattern_lowercase)
        }
    }
}

/// Apply filters to a vector of log references, returning owned logs that pass all filters
pub fn apply_filters(logs: Vec<&LogLine>, filters: &[Filter]) -> Vec<LogLine> {
    if filters.is_empty() {
        return logs.into_iter().map(|log| (*log).clone()).collect();
    }

    logs.into_iter()
        .filter(|log| {
            let line_lower = log.line_lowercase();
            // First, check exclude filters - if any match, exclude the log
            for filter in filters {
                if matches!(filter.filter_type, FilterType::Exclude) {
                    if filter.matches_lowercase(line_lower) {
                        return false;
                    }
                }
            }
            // Then, check include filters - if there are any, at least one must match
            let include_filters: Vec<_> = filters
                .iter()
                .filter(|f| matches!(f.filter_type, FilterType::Include))
                .collect();
            if include_filters.is_empty() {
                return true;
            }
            include_filters.iter().any(|filter| filter.matches_lowercase(line_lower))
        })
        .map(|log| (*log).clone())
        .collect()
}
