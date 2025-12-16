use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod buffer;
pub mod file;

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
}

impl LogLine {
    pub fn new(source: LogSource, line: String) -> Self {
        let now = Local::now();
        Self {
            id: NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: now,  // Will be updated by parser if found
            arrival_time: now,  // Capture arrival time
            source,
            line,
        }
    }

    pub fn memory_size(&self) -> usize {
        let mut size = std::mem::size_of::<LogLine>();

        size += self.line.capacity();

        match &self.source {
            LogSource::ProcessStdout(name) => size += name.capacity(),
            LogSource::ProcessStderr(name) => size += name.capacity(),
            LogSource::File { process_name, path } => {
                size += process_name.capacity();
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
}

impl LogSource {
    pub fn process_name(&self) -> &str {
        match self {
            LogSource::ProcessStdout(name) => name,
            LogSource::ProcessStderr(name) => name,
            LogSource::File { process_name, .. } => process_name,
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
}
