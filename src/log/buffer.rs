use super::LogLine;
use std::collections::VecDeque;

/// A circular buffer for storing log lines
pub struct LogBuffer {
    logs: VecDeque<LogLine>,
    max_size: usize,
}

impl LogBuffer {
    /// Create a new log buffer with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Create a new log buffer with a default size of 10,000 lines
    pub fn new_default() -> Self {
        Self::new(10_000)
    }

    /// Add a log line to the buffer
    /// If the buffer is full, the oldest line is removed
    pub fn push(&mut self, log: LogLine) {
        if self.logs.len() >= self.max_size {
            self.logs.pop_front();
        }
        self.logs.push_back(log);
    }

    /// Get the last n log lines
    pub fn get_last(&self, n: usize) -> Vec<&LogLine> {
        self.logs
            .iter()
            .rev()
            .take(n)
            .rev()
            .collect()
    }

    /// Get all log lines
    pub fn get_all(&self) -> Vec<&LogLine> {
        self.logs.iter().collect()
    }

    /// Get the number of logs in the buffer
    pub fn len(&self) -> usize {
        self.logs.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.logs.is_empty()
    }

    /// Clear all logs from the buffer
    pub fn clear(&mut self) {
        self.logs.clear();
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
}
