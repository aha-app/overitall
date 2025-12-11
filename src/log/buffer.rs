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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogSource;

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
}
