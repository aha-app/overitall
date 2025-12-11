use chrono::{DateTime, Local};
use std::path::PathBuf;

pub mod buffer;
pub mod file;

// Re-export commonly used types
pub use buffer::LogBuffer;
pub use file::FileReader;

/// A log line with enhanced metadata
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Local>,
    pub arrival_time: DateTime<Local>,  // When log was received
    pub source: LogSource,
    pub line: String,
}

impl LogLine {
    pub fn new(source: LogSource, line: String) -> Self {
        let now = Local::now();
        // Strip ANSI escape codes for clean display
        let clean_line = strip_ansi_escapes::strip_str(&line);
        Self {
            timestamp: now,  // Will be updated by parser if found
            arrival_time: now,  // Capture arrival time
            source,
            line: clean_line,
        }
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
