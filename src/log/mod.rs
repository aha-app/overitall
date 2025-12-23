use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod buffer;
pub mod display;
pub mod file;
pub mod velocity;

pub use display::{condense_log_line, strip_ansi};
pub use velocity::LogVelocityTracker;

// Re-export commonly used types
pub use buffer::LogBuffer;
pub use file::FileReader;

/// Global counter for unique log line IDs
static NEXT_LOG_ID: AtomicU64 = AtomicU64::new(1);

/// A log line with enhanced metadata
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Unique ID for this log line, assigned on creation
    pub id: u64,
    pub timestamp: DateTime<Local>,
    pub arrival_time: DateTime<Local>,  // When log was received
    pub source: LogSource,
    pub line: String,
    /// Pre-computed lowercase version of line for case-insensitive matching
    line_lowercase: String,
    /// Pre-computed formatted timestamp (HH:MM:SS)
    formatted_timestamp: String,
    /// Pre-computed line with ANSI codes stripped
    stripped_line: String,
    /// Pre-computed condensed version of line (metadata collapsed)
    condensed_line: String,
    /// Pre-computed condensed line with ANSI codes stripped
    condensed_stripped_line: String,
}

impl LogLine {
    pub fn new(source: LogSource, line: String) -> Self {
        let now = Local::now();
        let line_lowercase = line.to_lowercase();
        let formatted_timestamp = now.format("%H:%M:%S").to_string();
        let stripped_line = strip_ansi(&line);
        let condensed_line = condense_log_line(&line);
        let condensed_stripped_line = strip_ansi(&condensed_line);
        Self {
            id: NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: now,  // Will be updated by parser if found
            arrival_time: now,  // Capture arrival time
            source,
            line,
            line_lowercase,
            formatted_timestamp,
            stripped_line,
            condensed_line,
            condensed_stripped_line,
        }
    }

    /// Create a log line with specific timestamp (for benchmarks and tests)
    #[allow(dead_code)]
    pub fn new_with_time(source: LogSource, line: String, time: DateTime<Local>) -> Self {
        let line_lowercase = line.to_lowercase();
        let formatted_timestamp = time.format("%H:%M:%S").to_string();
        let stripped_line = strip_ansi(&line);
        let condensed_line = condense_log_line(&line);
        let condensed_stripped_line = strip_ansi(&condensed_line);
        Self {
            id: NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: time,
            arrival_time: time,
            source,
            line,
            line_lowercase,
            formatted_timestamp,
            stripped_line,
            condensed_line,
            condensed_stripped_line,
        }
    }

    /// Get the pre-computed lowercase version of the line
    pub fn line_lowercase(&self) -> &str {
        &self.line_lowercase
    }

    /// Get the pre-computed formatted timestamp (HH:MM:SS)
    pub fn formatted_timestamp(&self) -> &str {
        &self.formatted_timestamp
    }

    /// Get the pre-computed line with ANSI codes stripped
    pub fn stripped_line(&self) -> &str {
        &self.stripped_line
    }

    /// Get the pre-computed condensed version of line
    pub fn condensed_line(&self) -> &str {
        &self.condensed_line
    }

    /// Get the pre-computed condensed line with ANSI codes stripped
    pub fn condensed_stripped_line(&self) -> &str {
        &self.condensed_stripped_line
    }

    pub fn memory_size(&self) -> usize {
        let mut size = std::mem::size_of::<LogLine>();

        size += self.line.capacity();
        size += self.line_lowercase.capacity();
        size += self.formatted_timestamp.capacity();
        size += self.stripped_line.capacity();
        size += self.condensed_line.capacity();
        size += self.condensed_stripped_line.capacity();

        match &self.source {
            LogSource::ProcessStdout(name) => size += name.capacity(),
            LogSource::ProcessStderr(name) => size += name.capacity(),
            LogSource::File { process_name, path } => {
                size += process_name.capacity();
                size += path.as_os_str().len();
            }
            LogSource::StandaloneFile { name, path } => {
                size += name.capacity();
                size += path.as_os_str().len();
            }
        }

        size
    }
}

/// Source of a log line
#[derive(Debug, Clone)]
pub enum LogSource {
    ProcessStdout(String),  // process name
    ProcessStderr(String),  // process name
    File {
        process_name: String,
        path: PathBuf,
    },
    StandaloneFile {
        name: String,
        path: PathBuf,
    },
}

#[allow(dead_code)]
impl LogSource {
    pub fn process_name(&self) -> &str {
        match self {
            LogSource::ProcessStdout(name) => name,
            LogSource::ProcessStderr(name) => name,
            LogSource::File { process_name, .. } => process_name,
            LogSource::StandaloneFile { name, .. } => name,
        }
    }

    pub fn is_stdout(&self) -> bool {
        matches!(self, LogSource::ProcessStdout(_))
    }

    pub fn is_stderr(&self) -> bool {
        matches!(self, LogSource::ProcessStderr(_))
    }

    pub fn is_file(&self) -> bool {
        matches!(self, LogSource::File { .. })
    }

    pub fn is_standalone_file(&self) -> bool {
        matches!(self, LogSource::StandaloneFile { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_source_process_name_for_stdout() {
        let source = LogSource::ProcessStdout("web".to_string());
        assert_eq!(source.process_name(), "web");
        assert!(source.is_stdout());
        assert!(!source.is_stderr());
        assert!(!source.is_file());
        assert!(!source.is_standalone_file());
    }

    #[test]
    fn test_log_source_process_name_for_stderr() {
        let source = LogSource::ProcessStderr("worker".to_string());
        assert_eq!(source.process_name(), "worker");
        assert!(!source.is_stdout());
        assert!(source.is_stderr());
        assert!(!source.is_file());
        assert!(!source.is_standalone_file());
    }

    #[test]
    fn test_log_source_process_name_for_file() {
        let source = LogSource::File {
            process_name: "web".to_string(),
            path: PathBuf::from("/var/log/web.log"),
        };
        assert_eq!(source.process_name(), "web");
        assert!(!source.is_stdout());
        assert!(!source.is_stderr());
        assert!(source.is_file());
        assert!(!source.is_standalone_file());
    }

    #[test]
    fn test_log_source_process_name_for_standalone_file() {
        let source = LogSource::StandaloneFile {
            name: "rails".to_string(),
            path: PathBuf::from("/var/log/rails.log"),
        };
        assert_eq!(source.process_name(), "rails");
        assert!(!source.is_stdout());
        assert!(!source.is_stderr());
        assert!(!source.is_file());
        assert!(source.is_standalone_file());
    }
}
