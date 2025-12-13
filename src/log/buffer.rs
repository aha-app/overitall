use super::LogLine;
use std::collections::VecDeque;

/// A circular buffer for storing log lines
pub struct LogBuffer {
    logs: VecDeque<LogLine>,
    max_size: usize,
    max_memory_bytes: usize,
    current_memory_bytes: usize,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: VecDeque::with_capacity(max_size),
            max_size,
            max_memory_bytes: usize::MAX,
            current_memory_bytes: 0,
        }
    }

    pub fn new_with_memory_limit(max_memory_mb: usize) -> Self {
        Self {
            logs: VecDeque::new(),
            max_size: usize::MAX,
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            current_memory_bytes: 0,
        }
    }

    /// Create a new log buffer with a default size of 10,000 lines
    pub fn new_default() -> Self {
        Self::new(10_000)
    }

    /// If the buffer is full, the oldest line is removed
    pub fn push(&mut self, log: LogLine) {
        let log_size = log.memory_size();

        while (self.current_memory_bytes + log_size > self.max_memory_bytes
            || self.logs.len() >= self.max_size)
            && !self.logs.is_empty()
        {
            if let Some(evicted) = self.logs.pop_front() {
                self.current_memory_bytes = self.current_memory_bytes.saturating_sub(evicted.memory_size());
            }
        }

        self.current_memory_bytes += log_size;
        self.logs.push_back(log);
    }

    pub fn get_last(&self, n: usize) -> Vec<&LogLine> {
        self.logs
            .iter()
            .rev()
            .take(n)
            .rev()
            .collect()
    }

    pub fn get_all(&self) -> Vec<&LogLine> {
        self.logs.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.logs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    pub fn clear(&mut self) {
        self.logs.clear();
        self.current_memory_bytes = 0;
    }

    /// Detect batches of logs based on arrival time proximity
    /// Returns a vector of (start_index, end_index) tuples for each batch
    /// Logs are grouped into the same batch if they arrive within window_ms milliseconds
    pub fn detect_batches(&self, window_ms: i64) -> Vec<(usize, usize)> {
        if self.logs.is_empty() {
            return vec![];
        }

        if self.logs.len() == 1 {
            return vec![(0, 0)];
        }

        let mut batches = Vec::new();
        let mut batch_start = 0;

        for i in 1..self.logs.len() {
            let time_diff = self.logs[i].arrival_time - self.logs[i - 1].arrival_time;
            if time_diff.num_milliseconds() > window_ms {
                // Gap detected - end current batch, start new one
                batches.push((batch_start, i - 1));
                batch_start = i;
            }
        }

        // Don't forget the last batch!
        batches.push((batch_start, self.logs.len() - 1));

        batches
    }

    /// Get logs from a specific batch
    /// Returns an empty vector if batch_id is out of range
    pub fn get_batch(&self, batch_id: usize, window_ms: i64) -> Vec<&LogLine> {
        let batches = self.detect_batches(window_ms);

        if batch_id >= batches.len() {
            return vec![];
        }

        let (start, end) = batches[batch_id];
        self.logs
            .iter()
            .skip(start)
            .take(end - start + 1)
            .collect()
    }

    pub fn get_memory_usage_bytes(&self) -> usize {
        self.current_memory_bytes
    }

    pub fn get_memory_usage_mb(&self) -> f64 {
        self.current_memory_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn get_memory_limit_mb(&self) -> usize {
        self.max_memory_bytes / (1024 * 1024)
    }

    pub fn get_memory_usage_percent(&self) -> f64 {
        if self.max_memory_bytes == usize::MAX {
            0.0
        } else {
            (self.current_memory_bytes as f64 / self.max_memory_bytes as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;
    use chrono::{Local, TimeZone};

    #[test]
    fn test_buffer_push() {
        let mut buffer = LogBuffer::new(3);

        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line1".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line2".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line3".into()));

        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_buffer_circular() {
        let mut buffer = LogBuffer::new(2);

        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line1".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line2".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line3".into()));

        // Should only have 2 lines, and "line1" should be dropped
        assert_eq!(buffer.len(), 2);
        let logs = buffer.get_all();
        assert_eq!(logs[0].line, "line2");
        assert_eq!(logs[1].line, "line3");
    }

    #[test]
    fn test_buffer_get_last() {
        let mut buffer = LogBuffer::new(5);

        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line1".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line2".into()));
        buffer.push(LogLine::new(LogSource::ProcessStdout("test".into()), "line3".into()));

        let last_two = buffer.get_last(2);
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0].line, "line2");
        assert_eq!(last_two[1].line, "line3");
    }

    #[test]
    fn test_batch_detection_time_window() {
        let mut buffer = LogBuffer::new(100);

        // Create logs with specific arrival times
        let mut log1 = LogLine::new(LogSource::ProcessStdout("web".into()), "First batch line 1".into());
        log1.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

        let mut log2 = LogLine::new(LogSource::ProcessStdout("web".into()), "First batch line 2".into());
        log2.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(50);

        let mut log3 = LogLine::new(LogSource::ProcessStdout("web".into()), "Second batch line 1".into());
        log3.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 1).unwrap(); // 1 second later

        let mut log4 = LogLine::new(LogSource::ProcessStdout("worker".into()), "Second batch line 2".into());
        log4.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 1).unwrap()
            + chrono::Duration::milliseconds(80);

        buffer.push(log1);
        buffer.push(log2);
        buffer.push(log3);
        buffer.push(log4);

        let batches = buffer.detect_batches(100); // 100ms window
        assert_eq!(batches.len(), 2, "Should detect 2 batches");
        assert_eq!(batches[0], (0, 1), "First batch: lines 0-1");
        assert_eq!(batches[1], (2, 3), "Second batch: lines 2-3");
    }

    #[test]
    fn test_batch_detection_single_batch() {
        let mut buffer = LogBuffer::new(100);

        // All logs within window - should be one batch
        let mut log1 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 1".into());
        log1.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

        let mut log2 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 2".into());
        log2.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(50);

        let mut log3 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 3".into());
        log3.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(90);

        buffer.push(log1);
        buffer.push(log2);
        buffer.push(log3);

        let batches = buffer.detect_batches(100); // 100ms window
        assert_eq!(batches.len(), 1, "Should detect 1 batch");
        assert_eq!(batches[0], (0, 2), "Single batch: lines 0-2");
    }

    #[test]
    fn test_batch_detection_empty_buffer() {
        let buffer = LogBuffer::new(100);
        let batches = buffer.detect_batches(100);
        assert_eq!(batches.len(), 0, "Empty buffer should have no batches");
    }

    #[test]
    fn test_batch_detection_single_line() {
        let mut buffer = LogBuffer::new(100);
        buffer.push(LogLine::new(LogSource::ProcessStdout("web".into()), "Single line".into()));

        let batches = buffer.detect_batches(100);
        assert_eq!(batches.len(), 1, "Single line should be one batch");
        assert_eq!(batches[0], (0, 0), "Batch contains only line 0");
    }

    #[test]
    fn test_batch_detection_all_separate() {
        let mut buffer = LogBuffer::new(100);

        // Each log separated by more than window
        let mut log1 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 1".into());
        log1.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

        let mut log2 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 2".into());
        log2.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(200);

        let mut log3 = LogLine::new(LogSource::ProcessStdout("web".into()), "Line 3".into());
        log3.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(500);

        buffer.push(log1);
        buffer.push(log2);
        buffer.push(log3);

        let batches = buffer.detect_batches(100); // 100ms window
        assert_eq!(batches.len(), 3, "Should detect 3 separate batches");
        assert_eq!(batches[0], (0, 0), "First batch: line 0");
        assert_eq!(batches[1], (1, 1), "Second batch: line 1");
        assert_eq!(batches[2], (2, 2), "Third batch: line 2");
    }

    #[test]
    fn test_get_batch() {
        let mut buffer = LogBuffer::new(100);

        // Create two batches
        let mut log1 = LogLine::new(LogSource::ProcessStdout("web".into()), "Batch 0 Line 1".into());
        log1.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

        let mut log2 = LogLine::new(LogSource::ProcessStdout("web".into()), "Batch 0 Line 2".into());
        log2.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(50);

        let mut log3 = LogLine::new(LogSource::ProcessStdout("web".into()), "Batch 1 Line 1".into());
        log3.arrival_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 1).unwrap();

        buffer.push(log1);
        buffer.push(log2);
        buffer.push(log3);

        // Get first batch
        let batch0 = buffer.get_batch(0, 100);
        assert_eq!(batch0.len(), 2, "First batch should have 2 lines");
        assert_eq!(batch0[0].line, "Batch 0 Line 1");
        assert_eq!(batch0[1].line, "Batch 0 Line 2");

        // Get second batch
        let batch1 = buffer.get_batch(1, 100);
        assert_eq!(batch1.len(), 1, "Second batch should have 1 line");
        assert_eq!(batch1[0].line, "Batch 1 Line 1");

        // Get invalid batch
        let batch2 = buffer.get_batch(2, 100);
        assert_eq!(batch2.len(), 0, "Invalid batch should return empty vec");
    }

    #[test]
    fn test_log_line_memory_size() {
        let log = LogLine::new(LogSource::ProcessStdout("test".into()), "test line".into());
        let size = log.memory_size();

        assert!(size > 0, "Memory size should be greater than 0");
        assert!(size >= std::mem::size_of::<LogLine>(), "Size should at least include struct size");
    }

    #[test]
    fn test_buffer_memory_tracking() {
        let mut buffer = LogBuffer::new_with_memory_limit(1);

        assert_eq!(buffer.get_memory_usage_bytes(), 0, "Initial memory should be 0");

        let log1 = LogLine::new(LogSource::ProcessStdout("test".into()), "line1".into());
        let log1_size = log1.memory_size();
        buffer.push(log1);

        assert_eq!(buffer.get_memory_usage_bytes(), log1_size, "Memory should match log size");
        assert!(buffer.len() == 1, "Should have 1 log");
    }

    #[test]
    fn test_buffer_eviction_on_memory_limit() {
        let mut buffer = LogBuffer::new_with_memory_limit(1);

        let large_content = "x".repeat(50_000);
        let mut logs_pushed = 0;
        for i in 0..100 {
            let log = LogLine::new(
                LogSource::ProcessStdout("test".into()),
                format!("{} - log {}", large_content, i)
            );
            buffer.push(log);
            logs_pushed += 1;
        }

        assert_eq!(logs_pushed, 100, "Should have pushed 100 logs");
        assert!(buffer.len() < 100, "Should have evicted old logs due to memory limit");
        assert!(buffer.get_memory_usage_bytes() <= 1 * 1024 * 1024, "Memory usage should not exceed 1 MB");
    }

    #[test]
    fn test_buffer_single_large_log() {
        let mut buffer = LogBuffer::new_with_memory_limit(1);

        let huge_content = "x".repeat(2 * 1024 * 1024);
        let huge_log = LogLine::new(LogSource::ProcessStdout("test".into()), huge_content);

        buffer.push(huge_log);

        assert_eq!(buffer.len(), 1, "Should still accept single large log");
        assert!(buffer.get_memory_usage_bytes() > 1 * 1024 * 1024, "Large log exceeds limit but is kept");
    }

    #[test]
    fn test_buffer_stats_methods() {
        let mut buffer = LogBuffer::new_with_memory_limit(50);

        assert_eq!(buffer.get_memory_limit_mb(), 50, "Memory limit should be 50 MB");
        assert_eq!(buffer.get_memory_usage_mb(), 0.0, "Initial usage should be 0 MB");
        assert_eq!(buffer.get_memory_usage_percent(), 0.0, "Initial percent should be 0%");

        for i in 0..10 {
            let log = LogLine::new(
                LogSource::ProcessStdout("test".into()),
                format!("Log line {}", i)
            );
            buffer.push(log);
        }

        assert!(buffer.get_memory_usage_mb() > 0.0, "Usage should be > 0 MB after adding logs");
        assert!(buffer.get_memory_usage_percent() > 0.0, "Percent should be > 0% after adding logs");
        assert!(buffer.get_memory_usage_percent() < 100.0, "Percent should be < 100%");
    }

    #[test]
    fn test_buffer_clear_resets_memory() {
        let mut buffer = LogBuffer::new_with_memory_limit(10);

        for i in 0..5 {
            let log = LogLine::new(
                LogSource::ProcessStdout("test".into()),
                format!("Log line {}", i)
            );
            buffer.push(log);
        }

        assert!(buffer.get_memory_usage_bytes() > 0, "Should have memory usage");

        buffer.clear();

        assert_eq!(buffer.get_memory_usage_bytes(), 0, "Memory should be 0 after clear");
        assert_eq!(buffer.len(), 0, "Buffer should be empty after clear");
    }
}
