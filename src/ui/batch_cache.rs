use crate::log::LogLine;
use crate::ui::batch::detect_batches_from_logs;

#[derive(Clone, PartialEq, Eq)]
pub struct BatchCacheKey {
    log_signature: (usize, Option<u64>, Option<u64>),
    batch_window_ms: i64,
    filter_count: usize,
    search_pattern: String,
    hidden_count: usize,
    trace_filter_active: bool,
    using_snapshot: bool,
}

impl BatchCacheKey {
    pub fn from_context(
        logs: &[&LogLine],
        batch_window_ms: i64,
        filter_count: usize,
        search_pattern: String,
        hidden_count: usize,
        trace_filter_active: bool,
        using_snapshot: bool,
    ) -> Self {
        let first_id = logs.first().map(|l| l.id);
        let last_id = logs.last().map(|l| l.id);

        Self {
            log_signature: (logs.len(), first_id, last_id),
            batch_window_ms,
            filter_count,
            search_pattern,
            hidden_count,
            trace_filter_active,
            using_snapshot,
        }
    }
}

pub struct BatchCache {
    key: Option<BatchCacheKey>,
    batches: Vec<(usize, usize)>,
    pub hits: u64,
    pub misses: u64,
}

impl BatchCache {
    pub fn new() -> Self {
        Self {
            key: None,
            batches: Vec::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get_or_compute(
        &mut self,
        logs: &[&LogLine],
        window_ms: i64,
        current_key: BatchCacheKey,
    ) -> &Vec<(usize, usize)> {
        if self.key.as_ref() != Some(&current_key) {
            self.batches = detect_batches_from_logs(logs, window_ms);
            self.key = Some(current_key);
            self.misses += 1;
        } else {
            self.hits += 1;
        }
        &self.batches
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn invalidate(&mut self) {
        self.key = None;
    }
}

impl Default for BatchCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;
    use chrono::{Duration, Local};

    fn create_test_logs(count: usize) -> Vec<LogLine> {
        let base_time = Local::now();
        (0..count)
            .map(|i| {
                let arrival = base_time + Duration::milliseconds((i as i64) * 10);
                LogLine {
                    id: i as u64,
                    line: format!("Log line {}", i),
                    timestamp: arrival,
                    arrival_time: arrival,
                    source: LogSource::ProcessStdout("test".to_string()),
                }
            })
            .collect()
    }

    #[test]
    fn test_cache_hit() {
        let mut cache = BatchCache::new();
        let logs = create_test_logs(100);
        let refs: Vec<&LogLine> = logs.iter().collect();

        let key = BatchCacheKey::from_context(&refs, 100, 0, String::new(), 0, false, false);

        // First call - cache miss
        let batch_count = cache.get_or_compute(&refs, 100, key.clone()).len();
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 0);

        // Second call with same key - cache hit
        let batch_count2 = cache.get_or_compute(&refs, 100, key.clone()).len();
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 1);
        assert_eq!(batch_count2, batch_count);
    }

    #[test]
    fn test_cache_invalidation_on_window_change() {
        let mut cache = BatchCache::new();
        let logs = create_test_logs(100);
        let refs: Vec<&LogLine> = logs.iter().collect();

        let key1 = BatchCacheKey::from_context(&refs, 100, 0, String::new(), 0, false, false);
        let key2 = BatchCacheKey::from_context(&refs, 200, 0, String::new(), 0, false, false);

        // First call with window 100
        let _ = cache.get_or_compute(&refs, 100, key1);
        assert_eq!(cache.misses, 1);

        // Second call with different window - cache miss
        let _ = cache.get_or_compute(&refs, 200, key2);
        assert_eq!(cache.misses, 2);
    }

    #[test]
    fn test_cache_invalidation_on_log_change() {
        let mut cache = BatchCache::new();
        let logs1 = create_test_logs(100);
        let refs1: Vec<&LogLine> = logs1.iter().collect();

        let key1 = BatchCacheKey::from_context(&refs1, 100, 0, String::new(), 0, false, false);
        let _ = cache.get_or_compute(&refs1, 100, key1);
        assert_eq!(cache.misses, 1);

        // Add more logs
        let logs2 = create_test_logs(150);
        let refs2: Vec<&LogLine> = logs2.iter().collect();
        let key2 = BatchCacheKey::from_context(&refs2, 100, 0, String::new(), 0, false, false);

        // Should miss because log count changed
        let _ = cache.get_or_compute(&refs2, 100, key2);
        assert_eq!(cache.misses, 2);
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = BatchCache::new();
        let logs = create_test_logs(100);
        let refs: Vec<&LogLine> = logs.iter().collect();
        let key = BatchCacheKey::from_context(&refs, 100, 0, String::new(), 0, false, false);

        // 1 miss
        let _ = cache.get_or_compute(&refs, 100, key.clone());
        // 3 hits
        let _ = cache.get_or_compute(&refs, 100, key.clone());
        let _ = cache.get_or_compute(&refs, 100, key.clone());
        let _ = cache.get_or_compute(&refs, 100, key.clone());

        assert_eq!(cache.hit_rate(), 0.75); // 3 hits / 4 total
    }
}
