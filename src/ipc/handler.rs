use serde_json::{json, Value};

use super::protocol::{IpcRequest, IpcResponse};
use super::state::StateSnapshot;

/// Handles IPC commands from CLI clients
///
/// This handler processes incoming requests and returns appropriate responses.
/// It's designed to be simple and stateless for basic commands like ping/status.
pub struct IpcCommandHandler {
    version: String,
}

impl IpcCommandHandler {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    pub fn handle(&self, request: &IpcRequest, state: Option<&StateSnapshot>) -> IpcResponse {
        match request.command.as_str() {
            "ping" => self.handle_ping(),
            "status" => self.handle_status(&request.args, state),
            "processes" => self.handle_processes(state),
            "logs" => self.handle_logs(&request.args, state),
            _ => IpcResponse::err(format!("unknown command: {}", request.command)),
        }
    }

    fn handle_ping(&self) -> IpcResponse {
        IpcResponse::ok(json!({"pong": true}))
    }

    fn handle_status(&self, _args: &Value, state: Option<&StateSnapshot>) -> IpcResponse {
        match state {
            Some(snapshot) => {
                // Enhanced status with full state information
                IpcResponse::ok(json!({
                    "version": self.version,
                    "running": true,
                    "process_count": snapshot.processes.len(),
                    "filter_count": snapshot.filter_count,
                    "log_count": snapshot.log_count,
                    "auto_scroll": snapshot.auto_scroll,
                    "trace_recording": snapshot.trace_recording,
                    "view_mode": {
                        "frozen": snapshot.view_mode.frozen,
                        "batch_view": snapshot.view_mode.batch_view,
                        "trace_filter": snapshot.view_mode.trace_filter,
                        "compact": snapshot.view_mode.compact
                    },
                    "buffer": {
                        "bytes": snapshot.buffer_stats.buffer_bytes,
                        "max_bytes": snapshot.buffer_stats.max_buffer_bytes,
                        "usage_percent": snapshot.buffer_stats.usage_percent
                    }
                }))
            }
            None => {
                // Basic status when no state available (for backwards compatibility)
                IpcResponse::ok(json!({
                    "version": self.version,
                    "running": true
                }))
            }
        }
    }

    fn handle_processes(&self, state: Option<&StateSnapshot>) -> IpcResponse {
        match state {
            Some(snapshot) => {
                let processes: Vec<Value> = snapshot
                    .processes
                    .iter()
                    .map(|p| {
                        json!({
                            "name": p.name,
                            "status": p.status,
                            "error": p.error
                        })
                    })
                    .collect();
                IpcResponse::ok(json!({ "processes": processes }))
            }
            None => {
                // No state available - return empty list
                IpcResponse::ok(json!({ "processes": [] }))
            }
        }
    }

    fn handle_logs(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcResponse {
        // Parse optional limit and offset from args
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100);
        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);

        match state {
            Some(snapshot) => {
                // Apply offset and limit to recent_logs
                let logs: Vec<Value> = snapshot
                    .recent_logs
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .map(|log| {
                        json!({
                            "id": log.id,
                            "process": log.process,
                            "content": log.content,
                            "timestamp": log.timestamp,
                            "batch_id": log.batch_id
                        })
                    })
                    .collect();

                IpcResponse::ok(json!({
                    "logs": logs,
                    "total": snapshot.total_log_lines,
                    "offset": offset,
                    "limit": limit
                }))
            }
            None => {
                // No state available - return empty list
                IpcResponse::ok(json!({
                    "logs": [],
                    "total": 0,
                    "offset": offset,
                    "limit": limit
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler() -> IpcCommandHandler {
        IpcCommandHandler::new("0.1.0-test")
    }

    #[test]
    fn ping_returns_pong() {
        let handler = test_handler();
        let request = IpcRequest::new("ping");
        let response = handler.handle(&request, None);

        assert!(response.success);
        assert_eq!(response.result, Some(json!({"pong": true})));
        assert!(response.error.is_none());
    }

    #[test]
    fn status_returns_version_and_running() {
        let handler = test_handler();
        let request = IpcRequest::new("status");
        let response = handler.handle(&request, None);

        assert!(response.success);
        let result = response.result.unwrap();
        assert_eq!(result["version"], "0.1.0-test");
        assert_eq!(result["running"], true);
        assert!(response.error.is_none());
    }

    #[test]
    fn unknown_command_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("nonexistent");
        let response = handler.handle(&request, None);

        assert!(!response.success);
        assert!(response.result.is_none());
        assert_eq!(response.error, Some("unknown command: nonexistent".to_string()));
    }

    #[test]
    fn handler_uses_provided_version() {
        let handler = IpcCommandHandler::new("1.2.3");
        let request = IpcRequest::new("status");
        let response = handler.handle(&request, None);

        let result = response.result.unwrap();
        assert_eq!(result["version"], "1.2.3");
    }

    #[test]
    fn ping_with_args_ignores_args() {
        let handler = test_handler();
        let request = IpcRequest::with_args("ping", json!({"ignored": "data"}));
        let response = handler.handle(&request, None);

        assert!(response.success);
        assert_eq!(response.result, Some(json!({"pong": true})));
    }

    #[test]
    fn status_with_args_ignores_args() {
        let handler = test_handler();
        let request = IpcRequest::with_args("status", json!({"verbose": true}));
        let response = handler.handle(&request, None);

        assert!(response.success);
        let result = response.result.unwrap();
        assert_eq!(result["version"], "0.1.0-test");
    }

    #[test]
    fn status_with_state_returns_enhanced_info() {
        use super::super::state::{BufferStats, ProcessInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("status");

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
            filter_count: 3,
            active_filters: vec![],
            search_pattern: Some("error".to_string()),
            view_mode: ViewModeInfo {
                frozen: true,
                batch_view: false,
                trace_filter: true,
                trace_selection: false,
                compact: true,
            },
            auto_scroll: false,
            log_count: 1500,
            buffer_stats: BufferStats {
                buffer_bytes: 5000000,
                max_buffer_bytes: 52428800,
                usage_percent: 9.54,
            },
            trace_recording: true,
            active_trace_id: Some("abc123".to_string()),
            recent_logs: Vec::new(),
            total_log_lines: 1500,
        };

        let response = handler.handle(&request, Some(&snapshot));

        assert!(response.success);
        let result = response.result.unwrap();

        // Basic fields
        assert_eq!(result["version"], "0.1.0-test");
        assert_eq!(result["running"], true);

        // Enhanced fields from state
        assert_eq!(result["process_count"], 2);
        assert_eq!(result["filter_count"], 3);
        assert_eq!(result["log_count"], 1500);
        assert_eq!(result["auto_scroll"], false);
        assert_eq!(result["trace_recording"], true);

        // View mode
        assert_eq!(result["view_mode"]["frozen"], true);
        assert_eq!(result["view_mode"]["batch_view"], false);
        assert_eq!(result["view_mode"]["trace_filter"], true);
        assert_eq!(result["view_mode"]["compact"], true);

        // Buffer stats
        assert_eq!(result["buffer"]["bytes"], 5000000);
        assert_eq!(result["buffer"]["max_bytes"], 52428800);
    }

    #[test]
    fn processes_without_state_returns_empty_list() {
        let handler = test_handler();
        let request = IpcRequest::new("processes");
        let response = handler.handle(&request, None);

        assert!(response.success);
        let result = response.result.unwrap();
        let processes = result["processes"].as_array().unwrap();
        assert!(processes.is_empty());
    }

    #[test]
    fn processes_with_state_returns_process_list() {
        use super::super::state::{BufferStats, ProcessInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("processes");

        let snapshot = StateSnapshot {
            processes: vec![
                ProcessInfo {
                    name: "web".to_string(),
                    status: "running".to_string(),
                    error: None,
                },
                ProcessInfo {
                    name: "worker".to_string(),
                    status: "failed".to_string(),
                    error: Some("Exit code: 1".to_string()),
                },
            ],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo::default(),
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
            recent_logs: Vec::new(),
            total_log_lines: 0,
        };

        let response = handler.handle(&request, Some(&snapshot));

        assert!(response.success);
        let result = response.result.unwrap();
        let processes = result["processes"].as_array().unwrap();

        assert_eq!(processes.len(), 2);

        assert_eq!(processes[0]["name"], "web");
        assert_eq!(processes[0]["status"], "running");
        assert!(processes[0]["error"].is_null());

        assert_eq!(processes[1]["name"], "worker");
        assert_eq!(processes[1]["status"], "failed");
        assert_eq!(processes[1]["error"], "Exit code: 1");
    }

    #[test]
    fn logs_without_state_returns_empty_list() {
        let handler = test_handler();
        let request = IpcRequest::new("logs");
        let response = handler.handle(&request, None);

        assert!(response.success);
        let result = response.result.unwrap();
        let logs = result["logs"].as_array().unwrap();
        assert!(logs.is_empty());
        assert_eq!(result["total"], 0);
        assert_eq!(result["offset"], 0);
        assert_eq!(result["limit"], 100);
    }

    #[test]
    fn logs_with_state_returns_log_list() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("logs");

        let snapshot = StateSnapshot {
            processes: vec![],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo::default(),
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
            recent_logs: vec![
                LogLineInfo {
                    id: 1,
                    process: "web".to_string(),
                    content: "Server started".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: Some(1),
                },
                LogLineInfo {
                    id: 2,
                    process: "worker".to_string(),
                    content: "Processing job".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 1500,
        };

        let response = handler.handle(&request, Some(&snapshot));

        assert!(response.success);
        let result = response.result.unwrap();
        let logs = result["logs"].as_array().unwrap();

        assert_eq!(logs.len(), 2);
        assert_eq!(result["total"], 1500);
        assert_eq!(result["offset"], 0);
        assert_eq!(result["limit"], 100);

        assert_eq!(logs[0]["id"], 1);
        assert_eq!(logs[0]["process"], "web");
        assert_eq!(logs[0]["content"], "Server started");
        assert_eq!(logs[0]["timestamp"], "2025-12-17T10:00:00Z");
        assert_eq!(logs[0]["batch_id"], 1);

        assert_eq!(logs[1]["id"], 2);
        assert_eq!(logs[1]["process"], "worker");
        assert!(logs[1]["batch_id"].is_null());
    }

    #[test]
    fn logs_with_limit_and_offset() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("logs", json!({"limit": 1, "offset": 1}));

        let snapshot = StateSnapshot {
            processes: vec![],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo::default(),
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
            recent_logs: vec![
                LogLineInfo {
                    id: 1,
                    process: "web".to_string(),
                    content: "First log".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 2,
                    process: "web".to_string(),
                    content: "Second log".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 3,
                    process: "web".to_string(),
                    content: "Third log".to_string(),
                    timestamp: "2025-12-17T10:00:02Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 3,
        };

        let response = handler.handle(&request, Some(&snapshot));

        assert!(response.success);
        let result = response.result.unwrap();
        let logs = result["logs"].as_array().unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(result["offset"], 1);
        assert_eq!(result["limit"], 1);
        assert_eq!(logs[0]["id"], 2);
        assert_eq!(logs[0]["content"], "Second log");
    }
}
