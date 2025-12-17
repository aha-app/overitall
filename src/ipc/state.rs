// StateSnapshot types for IPC commands that need application state
// These are simple data structures passed to the IPC handler for state-dependent commands

use serde::{Deserialize, Serialize};

/// Snapshot of application state for IPC commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Process information
    pub processes: Vec<ProcessInfo>,

    /// Number of active filters
    pub filter_count: usize,

    /// Active filter details
    pub active_filters: Vec<FilterInfo>,

    /// Current search pattern if any
    pub search_pattern: Option<String>,

    /// View mode information
    pub view_mode: ViewModeInfo,

    /// Whether auto-scroll is enabled
    pub auto_scroll: bool,

    /// Total number of log lines
    pub log_count: usize,

    /// Buffer statistics
    pub buffer_stats: BufferStats,

    /// Whether trace recording is active
    pub trace_recording: bool,

    /// Active trace ID if any
    pub active_trace_id: Option<String>,
}

/// Information about a single process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process name from Procfile
    pub name: String,

    /// Current status: "running", "stopped", "failed", "terminating", "restarting"
    pub status: String,

    /// Error message for failed processes
    pub error: Option<String>,
}

/// Information about a filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterInfo {
    /// The filter pattern
    pub pattern: String,

    /// Filter type: "include" or "exclude"
    pub filter_type: String,
}

/// View mode state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewModeInfo {
    /// Whether view is frozen (scrolled away from tail)
    pub frozen: bool,

    /// Whether batch view is active
    pub batch_view: bool,

    /// Whether trace filter is active
    pub trace_filter: bool,

    /// Whether trace selection overlay is active
    pub trace_selection: bool,

    /// Whether compact mode is enabled
    pub compact: bool,
}

/// Buffer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStats {
    /// Current buffer size in bytes
    pub buffer_bytes: usize,

    /// Maximum buffer size in bytes
    pub max_buffer_bytes: usize,

    /// Buffer usage as a percentage (0.0 - 100.0)
    pub usage_percent: f64,
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            filter_count: 0,
            active_filters: Vec::new(),
            search_pattern: None,
            view_mode: ViewModeInfo::default(),
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
        }
    }
}

impl Default for ViewModeInfo {
    fn default() -> Self {
        Self {
            frozen: false,
            batch_view: false,
            trace_filter: false,
            trace_selection: false,
            compact: false,
        }
    }
}

impl Default for BufferStats {
    fn default() -> Self {
        Self {
            buffer_bytes: 0,
            max_buffer_bytes: 0,
            usage_percent: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_info_serialization() {
        let info = ProcessInfo {
            name: "web".to_string(),
            status: "running".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: ProcessInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "web");
        assert_eq!(parsed.status, "running");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_process_info_with_error() {
        let info = ProcessInfo {
            name: "worker".to_string(),
            status: "failed".to_string(),
            error: Some("command not found: node".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: ProcessInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "worker");
        assert_eq!(parsed.status, "failed");
        assert_eq!(parsed.error, Some("command not found: node".to_string()));
    }

    #[test]
    fn test_filter_info_serialization() {
        let filter = FilterInfo {
            pattern: "error".to_string(),
            filter_type: "include".to_string(),
        };

        let json = serde_json::to_string(&filter).unwrap();
        let parsed: FilterInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pattern, "error");
        assert_eq!(parsed.filter_type, "include");
    }

    #[test]
    fn test_view_mode_info_serialization() {
        let view = ViewModeInfo {
            frozen: true,
            batch_view: false,
            trace_filter: true,
            trace_selection: false,
            compact: true,
        };

        let json = serde_json::to_string(&view).unwrap();
        let parsed: ViewModeInfo = serde_json::from_str(&json).unwrap();

        assert!(parsed.frozen);
        assert!(!parsed.batch_view);
        assert!(parsed.trace_filter);
        assert!(!parsed.trace_selection);
        assert!(parsed.compact);
    }

    #[test]
    fn test_buffer_stats_serialization() {
        let stats = BufferStats {
            buffer_bytes: 1024000,
            max_buffer_bytes: 10240000,
            usage_percent: 10.0,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: BufferStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.buffer_bytes, 1024000);
        assert_eq!(parsed.max_buffer_bytes, 10240000);
        assert!((parsed.usage_percent - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_state_snapshot_serialization() {
        let snapshot = StateSnapshot {
            processes: vec![
                ProcessInfo {
                    name: "web".to_string(),
                    status: "running".to_string(),
                    error: None,
                },
                ProcessInfo {
                    name: "worker".to_string(),
                    status: "stopped".to_string(),
                    error: None,
                },
            ],
            filter_count: 2,
            active_filters: vec![
                FilterInfo {
                    pattern: "error".to_string(),
                    filter_type: "include".to_string(),
                },
                FilterInfo {
                    pattern: "debug".to_string(),
                    filter_type: "exclude".to_string(),
                },
            ],
            search_pattern: Some("panic".to_string()),
            view_mode: ViewModeInfo {
                frozen: true,
                batch_view: false,
                trace_filter: false,
                trace_selection: false,
                compact: true,
            },
            auto_scroll: false,
            log_count: 1523,
            buffer_stats: BufferStats {
                buffer_bytes: 5120000,
                max_buffer_bytes: 10240000,
                usage_percent: 50.0,
            },
            trace_recording: true,
            active_trace_id: Some("abc123".to_string()),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: StateSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.processes.len(), 2);
        assert_eq!(parsed.processes[0].name, "web");
        assert_eq!(parsed.filter_count, 2);
        assert_eq!(parsed.active_filters.len(), 2);
        assert_eq!(parsed.search_pattern, Some("panic".to_string()));
        assert!(parsed.view_mode.frozen);
        assert!(parsed.view_mode.compact);
        assert!(!parsed.auto_scroll);
        assert_eq!(parsed.log_count, 1523);
        assert_eq!(parsed.buffer_stats.buffer_bytes, 5120000);
        assert!(parsed.trace_recording);
        assert_eq!(parsed.active_trace_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_state_snapshot_default() {
        let snapshot = StateSnapshot::default();

        assert!(snapshot.processes.is_empty());
        assert_eq!(snapshot.filter_count, 0);
        assert!(snapshot.active_filters.is_empty());
        assert!(snapshot.search_pattern.is_none());
        assert!(!snapshot.view_mode.frozen);
        assert!(snapshot.auto_scroll);
        assert_eq!(snapshot.log_count, 0);
        assert_eq!(snapshot.buffer_stats.buffer_bytes, 0);
        assert!(!snapshot.trace_recording);
        assert!(snapshot.active_trace_id.is_none());
    }

    #[test]
    fn test_state_snapshot_json_format() {
        let snapshot = StateSnapshot {
            processes: vec![ProcessInfo {
                name: "web".to_string(),
                status: "running".to_string(),
                error: None,
            }],
            filter_count: 1,
            active_filters: vec![FilterInfo {
                pattern: "info".to_string(),
                filter_type: "include".to_string(),
            }],
            search_pattern: None,
            view_mode: ViewModeInfo::default(),
            auto_scroll: true,
            log_count: 100,
            buffer_stats: BufferStats {
                buffer_bytes: 1000,
                max_buffer_bytes: 10000,
                usage_percent: 10.0,
            },
            trace_recording: false,
            active_trace_id: None,
        };

        let json = serde_json::to_string_pretty(&snapshot).unwrap();

        // Verify key fields are present in JSON output
        assert!(json.contains("\"processes\""));
        assert!(json.contains("\"filter_count\""));
        assert!(json.contains("\"active_filters\""));
        assert!(json.contains("\"view_mode\""));
        assert!(json.contains("\"auto_scroll\""));
        assert!(json.contains("\"log_count\""));
        assert!(json.contains("\"buffer_stats\""));
        assert!(json.contains("\"trace_recording\""));
    }
}
