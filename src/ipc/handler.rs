use serde_json::{json, Value};

use super::action::{IpcAction, IpcHandlerResult};
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

    pub fn handle(&self, request: &IpcRequest, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        match request.command.as_str() {
            "ping" => IpcHandlerResult::response_only(self.handle_ping()),
            "status" => IpcHandlerResult::response_only(self.handle_status(&request.args, state)),
            "processes" => IpcHandlerResult::response_only(self.handle_processes(state)),
            "logs" => IpcHandlerResult::response_only(self.handle_logs(&request.args, state)),
            "search" => self.handle_search(&request.args, state),
            "select" => self.handle_select(&request.args, state),
            "context" => self.handle_context(&request.args, state),
            "goto" => self.handle_goto(&request.args, state),
            "scroll" => self.handle_scroll(&request.args, state),
            "freeze" => self.handle_freeze(&request.args, state),
            "help" => IpcHandlerResult::response_only(self.handle_help()),
            "trace" => IpcHandlerResult::response_only(self.handle_trace(state)),
            _ => IpcHandlerResult::response_only(IpcResponse::err(format!(
                "unknown command: {}",
                request.command
            ))),
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

    fn handle_search(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // Pattern is required
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return IpcHandlerResult::response_only(IpcResponse::err(
                    "missing required argument: pattern".to_string(),
                ));
            }
        };

        // Parse optional arguments
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100);
        let case_sensitive = args
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Create actions to update TUI: set search pattern and disable auto-scroll
        // so the user sees the same frozen view as the CLI results
        let actions = vec![
            IpcAction::SetSearch {
                pattern: pattern.to_string(),
            },
            IpcAction::SetAutoScroll { enabled: false },
        ];

        match state {
            Some(snapshot) => {
                // Search through recent_logs in reverse order (newest first)
                // This ensures the LLM sees the most recent matches when debugging
                let pattern_lower = pattern.to_lowercase();

                let matches: Vec<Value> = snapshot
                    .recent_logs
                    .iter()
                    .rev() // Newest first
                    .filter(|log| {
                        if case_sensitive {
                            log.content.contains(pattern)
                        } else {
                            log.content.to_lowercase().contains(&pattern_lower)
                        }
                    })
                    .take(limit)
                    .map(|log| {
                        json!({
                            "id": log.id,
                            "process": log.process,
                            "content": log.content,
                            "timestamp": log.timestamp
                        })
                    })
                    .collect();

                let count = matches.len();

                IpcHandlerResult::with_actions(
                    IpcResponse::ok(json!({
                        "matches": matches,
                        "pattern": pattern,
                        "count": count,
                        "limit": limit
                    })),
                    actions,
                )
            }
            None => {
                // No state available - still emit action to update TUI, return empty results
                IpcHandlerResult::with_actions(
                    IpcResponse::ok(json!({
                        "matches": [],
                        "pattern": pattern,
                        "count": 0,
                        "limit": limit
                    })),
                    actions,
                )
            }
        }
    }

    fn handle_select(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // ID is required
        let id = match args.get("id").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return IpcHandlerResult::response_only(IpcResponse::err(
                    "missing required argument: id".to_string(),
                ));
            }
        };

        // Verify the log line exists in current state
        let line_exists = state
            .map(|s| s.recent_logs.iter().any(|log| log.id == id))
            .unwrap_or(false);

        if !line_exists {
            return IpcHandlerResult::response_only(IpcResponse::err(format!(
                "log line with id {} not found",
                id
            )));
        }

        // Emit action to select and expand the line, also disable auto-scroll
        let actions = vec![
            IpcAction::SelectAndExpandLine { id },
            IpcAction::SetAutoScroll { enabled: false },
        ];

        IpcHandlerResult::with_actions(
            IpcResponse::ok(json!({
                "selected": true,
                "id": id
            })),
            actions,
        )
    }

    fn handle_context(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // ID is required
        let id = match args.get("id").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return IpcHandlerResult::response_only(IpcResponse::err(
                    "missing required argument: id".to_string(),
                ));
            }
        };

        // Parse optional before/after counts (default: 5 lines each)
        let before = args
            .get("before")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);
        let after = args
            .get("after")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);

        match state {
            Some(snapshot) => {
                // Find the index of the target log line
                let target_idx = snapshot
                    .recent_logs
                    .iter()
                    .position(|log| log.id == id);

                match target_idx {
                    Some(idx) => {
                        // Calculate range with bounds checking
                        let start = idx.saturating_sub(before);
                        let end = (idx + after + 1).min(snapshot.recent_logs.len());

                        // Collect context lines
                        let context_lines: Vec<Value> = snapshot.recent_logs[start..end]
                            .iter()
                            .map(|log| {
                                json!({
                                    "id": log.id,
                                    "process": log.process,
                                    "content": log.content,
                                    "timestamp": log.timestamp,
                                    "is_target": log.id == id
                                })
                            })
                            .collect();

                        IpcHandlerResult::response_only(IpcResponse::ok(json!({
                            "target_id": id,
                            "before": before,
                            "after": after,
                            "lines": context_lines,
                            "count": context_lines.len()
                        })))
                    }
                    None => IpcHandlerResult::response_only(IpcResponse::err(format!(
                        "log line with id {} not found",
                        id
                    ))),
                }
            }
            None => IpcHandlerResult::response_only(IpcResponse::err(
                "no state available".to_string(),
            )),
        }
    }

    fn handle_goto(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // ID is required
        let id = match args.get("id").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return IpcHandlerResult::response_only(IpcResponse::err(
                    "missing required argument: id".to_string(),
                ));
            }
        };

        // Verify the log line exists in current state
        let line_exists = state
            .map(|s| s.recent_logs.iter().any(|log| log.id == id))
            .unwrap_or(false);

        if !line_exists {
            return IpcHandlerResult::response_only(IpcResponse::err(format!(
                "log line with id {} not found",
                id
            )));
        }

        // Emit actions to scroll to line and disable auto-scroll
        let actions = vec![
            IpcAction::ScrollToLine { id },
            IpcAction::SetAutoScroll { enabled: false },
        ];

        IpcHandlerResult::with_actions(
            IpcResponse::ok(json!({
                "scrolled_to": id
            })),
            actions,
        )
    }

    fn handle_scroll(&self, args: &Value, _state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // Direction is required
        let direction = match args.get("direction").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => {
                return IpcHandlerResult::response_only(IpcResponse::err(
                    "missing required argument: direction".to_string(),
                ));
            }
        };

        // Parse optional lines count (default: 20)
        let lines = args
            .get("lines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(20);

        // Determine action and auto_scroll setting based on direction
        let (action, auto_scroll) = match direction {
            "up" => (IpcAction::ScrollUp { lines }, false),
            "down" => (IpcAction::ScrollDown { lines }, false),
            "top" => (IpcAction::ScrollToTop, false),
            "bottom" => (IpcAction::SetAutoScroll { enabled: true }, true),
            _ => {
                return IpcHandlerResult::response_only(IpcResponse::err(format!(
                    "invalid direction: {}. Valid options: up, down, top, bottom",
                    direction
                )));
            }
        };

        let response = IpcResponse::ok(json!({
            "direction": direction,
            "lines": lines,
            "auto_scroll": auto_scroll
        }));

        // For bottom, we only emit SetAutoScroll. For others, emit the scroll action + disable auto-scroll
        if auto_scroll {
            IpcHandlerResult::with_actions(response, vec![action])
        } else {
            IpcHandlerResult::with_actions(
                response,
                vec![action, IpcAction::SetAutoScroll { enabled: false }],
            )
        }
    }

    fn handle_freeze(&self, args: &Value, state: Option<&StateSnapshot>) -> IpcHandlerResult {
        // Parse mode: on, off, or toggle (default: toggle)
        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("toggle");

        let frozen = match mode {
            "on" => true,
            "off" => false,
            "toggle" => {
                // Need current state to toggle - default to freezing if no state
                state.map(|s| !s.view_mode.frozen).unwrap_or(true)
            }
            _ => {
                return IpcHandlerResult::response_only(IpcResponse::err(format!(
                    "invalid mode: {}. Valid options: on, off, toggle",
                    mode
                )));
            }
        };

        IpcHandlerResult::with_actions(
            IpcResponse::ok(json!({
                "frozen": frozen,
                "mode": mode
            })),
            vec![IpcAction::SetFrozen { frozen }],
        )
    }

    fn handle_help(&self) -> IpcResponse {
        IpcResponse::ok(json!({
            "commands": [
                {
                    "name": "ping",
                    "description": "Check if TUI is running",
                    "args": []
                },
                {
                    "name": "status",
                    "description": "Get TUI status including version, process count, and buffer usage",
                    "args": []
                },
                {
                    "name": "processes",
                    "description": "List all processes and their current status",
                    "args": []
                },
                {
                    "name": "logs",
                    "description": "Get recent log lines from the buffer",
                    "args": [
                        {"name": "limit", "type": "number", "default": 100, "description": "Maximum number of lines to return"},
                        {"name": "offset", "type": "number", "default": 0, "description": "Number of lines to skip"}
                    ]
                },
                {
                    "name": "search",
                    "description": "Search log lines for a pattern and highlight in TUI",
                    "args": [
                        {"name": "pattern", "type": "string", "required": true, "description": "Search pattern (substring match)"},
                        {"name": "limit", "type": "number", "default": 100, "description": "Maximum matches to return"},
                        {"name": "case_sensitive", "type": "boolean", "default": false, "description": "Enable case-sensitive matching"}
                    ]
                },
                {
                    "name": "select",
                    "description": "Select a log line by ID and open expanded view in TUI",
                    "args": [
                        {"name": "id", "type": "number", "required": true, "description": "Log line ID to select"}
                    ]
                },
                {
                    "name": "context",
                    "description": "Get context lines around a specific log line",
                    "args": [
                        {"name": "id", "type": "number", "required": true, "description": "Log line ID"},
                        {"name": "before", "type": "number", "default": 5, "description": "Lines before target"},
                        {"name": "after", "type": "number", "default": 5, "description": "Lines after target"}
                    ]
                },
                {
                    "name": "goto",
                    "description": "Jump to a specific log line by ID (scrolls view without expanding)",
                    "args": [
                        {"name": "id", "type": "number", "required": true, "description": "Log line ID to scroll to"}
                    ]
                },
                {
                    "name": "scroll",
                    "description": "Scroll the log view up, down, to top, or to bottom",
                    "args": [
                        {"name": "direction", "type": "string", "required": true, "description": "Scroll direction: up, down, top, or bottom"},
                        {"name": "lines", "type": "number", "default": 20, "description": "Number of lines to scroll (for up/down)"}
                    ]
                },
                {
                    "name": "freeze",
                    "description": "Freeze or unfreeze the TUI display (pauses auto-scroll)",
                    "args": [
                        {"name": "mode", "type": "string", "default": "toggle", "description": "Mode: on, off, or toggle"}
                    ]
                },
                {
                    "name": "help",
                    "description": "List available IPC commands",
                    "args": []
                },
                {
                    "name": "trace",
                    "description": "Get trace recording status and active trace info",
                    "args": []
                }
            ],
            "version": self.version
        }))
    }

    fn handle_trace(&self, state: Option<&StateSnapshot>) -> IpcResponse {
        match state {
            Some(snapshot) => {
                IpcResponse::ok(json!({
                    "recording": snapshot.trace_recording,
                    "active_trace_id": snapshot.active_trace_id,
                    "trace_filter_active": snapshot.view_mode.trace_filter,
                    "trace_selection_active": snapshot.view_mode.trace_selection
                }))
            }
            None => {
                IpcResponse::ok(json!({
                    "recording": false,
                    "active_trace_id": null,
                    "trace_filter_active": false,
                    "trace_selection_active": false
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
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        assert_eq!(result.response.result, Some(json!({"pong": true})));
        assert!(result.response.error.is_none());
        assert!(result.actions.is_empty());
    }

    #[test]
    fn status_returns_version_and_running() {
        let handler = test_handler();
        let request = IpcRequest::new("status");
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["version"], "0.1.0-test");
        assert_eq!(data["running"], true);
        assert!(result.actions.is_empty());
    }

    #[test]
    fn unknown_command_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("nonexistent");
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.result.is_none());
        assert_eq!(
            result.response.error,
            Some("unknown command: nonexistent".to_string())
        );
        assert!(result.actions.is_empty());
    }

    #[test]
    fn handler_uses_provided_version() {
        let handler = IpcCommandHandler::new("1.2.3");
        let request = IpcRequest::new("status");
        let result = handler.handle(&request, None);

        let data = result.response.result.unwrap();
        assert_eq!(data["version"], "1.2.3");
    }

    #[test]
    fn ping_with_args_ignores_args() {
        let handler = test_handler();
        let request = IpcRequest::with_args("ping", json!({"ignored": "data"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        assert_eq!(result.response.result, Some(json!({"pong": true})));
    }

    #[test]
    fn status_with_args_ignores_args() {
        let handler = test_handler();
        let request = IpcRequest::with_args("status", json!({"verbose": true}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["version"], "0.1.0-test");
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

        let handler_result = handler.handle(&request, Some(&snapshot));

        assert!(handler_result.response.success);
        let data = handler_result.response.result.unwrap();

        // Basic fields
        assert_eq!(data["version"], "0.1.0-test");
        assert_eq!(data["running"], true);

        // Enhanced fields from state
        assert_eq!(data["process_count"], 2);
        assert_eq!(data["filter_count"], 3);
        assert_eq!(data["log_count"], 1500);
        assert_eq!(data["auto_scroll"], false);
        assert_eq!(data["trace_recording"], true);

        // View mode
        assert_eq!(data["view_mode"]["frozen"], true);
        assert_eq!(data["view_mode"]["batch_view"], false);
        assert_eq!(data["view_mode"]["trace_filter"], true);
        assert_eq!(data["view_mode"]["compact"], true);

        // Buffer stats
        assert_eq!(data["buffer"]["bytes"], 5000000);
        assert_eq!(data["buffer"]["max_bytes"], 52428800);
    }

    #[test]
    fn processes_without_state_returns_empty_list() {
        let handler = test_handler();
        let request = IpcRequest::new("processes");
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let processes = data["processes"].as_array().unwrap();
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

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let processes = data["processes"].as_array().unwrap();

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
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let logs = data["logs"].as_array().unwrap();
        assert!(logs.is_empty());
        assert_eq!(data["total"], 0);
        assert_eq!(data["offset"], 0);
        assert_eq!(data["limit"], 100);
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

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let logs = data["logs"].as_array().unwrap();

        assert_eq!(logs.len(), 2);
        assert_eq!(data["total"], 1500);
        assert_eq!(data["offset"], 0);
        assert_eq!(data["limit"], 100);

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

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let logs = data["logs"].as_array().unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(data["offset"], 1);
        assert_eq!(data["limit"], 1);
        assert_eq!(logs[0]["id"], 2);
        assert_eq!(logs[0]["content"], "Second log");
    }

    #[test]
    fn search_without_pattern_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("search");
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.is_some());
        assert!(result.response.error.unwrap().contains("pattern"));
        // No actions on error
        assert!(result.actions.is_empty());
    }

    #[test]
    fn search_without_state_returns_empty_matches() {
        let handler = test_handler();
        let request = IpcRequest::with_args("search", json!({"pattern": "error"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let matches = data["matches"].as_array().unwrap();
        assert!(matches.is_empty());
        assert_eq!(data["pattern"], "error");
        assert_eq!(data["count"], 0);
        assert_eq!(data["limit"], 100);

        // Should emit SetSearch and SetAutoScroll actions
        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetSearch { pattern } if pattern == "error"
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn search_with_state_finds_matches() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("search", json!({"pattern": "error"}));

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
                    batch_id: None,
                },
                LogLineInfo {
                    id: 2,
                    process: "web".to_string(),
                    content: "Error: connection failed".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 3,
                    process: "worker".to_string(),
                    content: "Processing job".to_string(),
                    timestamp: "2025-12-17T10:00:02Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 4,
                    process: "worker".to_string(),
                    content: "Job error: timeout".to_string(),
                    timestamp: "2025-12-17T10:00:03Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 4,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let matches = data["matches"].as_array().unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(data["pattern"], "error");
        assert_eq!(data["count"], 2);

        // Results are newest first
        assert_eq!(matches[0]["id"], 4);
        assert_eq!(matches[0]["content"], "Job error: timeout");
        assert_eq!(matches[1]["id"], 2);
        assert_eq!(matches[1]["content"], "Error: connection failed");

        // Should emit SetSearch and SetAutoScroll actions
        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetSearch { pattern } if pattern == "error"
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn search_case_sensitive() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request =
            IpcRequest::with_args("search", json!({"pattern": "Error", "case_sensitive": true}));

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
                    content: "Error: connection failed".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 2,
                    process: "worker".to_string(),
                    content: "Job error: timeout".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 2,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let matches = data["matches"].as_array().unwrap();

        // Only "Error" (capital E) should match
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["id"], 1);
        assert_eq!(matches[0]["content"], "Error: connection failed");

        // Action should have the exact pattern and disable auto-scroll
        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetSearch { pattern } if pattern == "Error"
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn search_with_limit() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("search", json!({"pattern": "log", "limit": 2}));

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
                    content: "Log line 1".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 2,
                    process: "web".to_string(),
                    content: "Log line 2".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 3,
                    process: "web".to_string(),
                    content: "Log line 3".to_string(),
                    timestamp: "2025-12-17T10:00:02Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 4,
                    process: "web".to_string(),
                    content: "Log line 4".to_string(),
                    timestamp: "2025-12-17T10:00:03Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 4,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        let matches = data["matches"].as_array().unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(data["limit"], 2);
        // Results are newest first
        assert_eq!(matches[0]["id"], 4);
        assert_eq!(matches[1]["id"], 3);
    }

    #[test]
    fn select_without_id_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("select");
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.is_some());
        assert!(result.response.error.unwrap().contains("id"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn select_with_nonexistent_id_returns_error() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("select", json!({"id": 999}));

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
            recent_logs: vec![LogLineInfo {
                id: 1,
                process: "web".to_string(),
                content: "Test log".to_string(),
                timestamp: "2025-12-17T10:00:00Z".to_string(),
                batch_id: None,
            }],
            total_log_lines: 1,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(!result.response.success);
        assert!(result.response.error.unwrap().contains("not found"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn select_with_valid_id_returns_success_and_actions() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("select", json!({"id": 42}));

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
                    id: 42,
                    process: "web".to_string(),
                    content: "The important log".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 43,
                    process: "worker".to_string(),
                    content: "Another log".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 2,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["selected"], true);
        assert_eq!(data["id"], 42);

        // Should emit SelectAndExpandLine and SetAutoScroll actions
        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SelectAndExpandLine { id } if *id == 42
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn help_returns_command_list() {
        let handler = test_handler();
        let request = IpcRequest::new("help");
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();

        // Check that commands array exists
        let commands = data["commands"].as_array().unwrap();
        assert!(!commands.is_empty());

        // Check that version is included
        assert_eq!(data["version"], "0.1.0-test");

        // Verify some expected commands are present
        let command_names: Vec<&str> = commands
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(command_names.contains(&"ping"));
        assert!(command_names.contains(&"status"));
        assert!(command_names.contains(&"processes"));
        assert!(command_names.contains(&"logs"));
        assert!(command_names.contains(&"search"));
        assert!(command_names.contains(&"select"));
        assert!(command_names.contains(&"context"));
        assert!(command_names.contains(&"help"));
        assert!(command_names.contains(&"trace"));

        // Check that no actions are emitted
        assert!(result.actions.is_empty());
    }

    #[test]
    fn help_includes_command_descriptions_and_args() {
        let handler = test_handler();
        let request = IpcRequest::new("help");
        let result = handler.handle(&request, None);

        let data = result.response.result.unwrap();
        let commands = data["commands"].as_array().unwrap();

        // Find the search command and verify its structure
        let search_cmd = commands
            .iter()
            .find(|c| c["name"].as_str() == Some("search"))
            .unwrap();

        assert!(search_cmd["description"].as_str().unwrap().len() > 0);
        let args = search_cmd["args"].as_array().unwrap();

        // Verify search has pattern, limit, and case_sensitive args
        let arg_names: Vec<&str> = args
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        assert!(arg_names.contains(&"pattern"));
        assert!(arg_names.contains(&"limit"));
        assert!(arg_names.contains(&"case_sensitive"));
    }

    #[test]
    fn trace_without_state_returns_defaults() {
        let handler = test_handler();
        let request = IpcRequest::new("trace");
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();

        assert_eq!(data["recording"], false);
        assert!(data["active_trace_id"].is_null());
        assert_eq!(data["trace_filter_active"], false);
        assert_eq!(data["trace_selection_active"], false);

        assert!(result.actions.is_empty());
    }

    #[test]
    fn trace_with_state_returns_trace_info() {
        use super::super::state::{BufferStats, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("trace");

        let snapshot = StateSnapshot {
            processes: vec![],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo {
                frozen: false,
                batch_view: false,
                trace_filter: true,
                trace_selection: false,
                compact: false,
            },
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: true,
            active_trace_id: Some("abc123def".to_string()),
            recent_logs: Vec::new(),
            total_log_lines: 0,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();

        assert_eq!(data["recording"], true);
        assert_eq!(data["active_trace_id"], "abc123def");
        assert_eq!(data["trace_filter_active"], true);
        assert_eq!(data["trace_selection_active"], false);

        assert!(result.actions.is_empty());
    }

    #[test]
    fn goto_without_id_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("goto");
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.is_some());
        assert!(result.response.error.unwrap().contains("id"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn goto_with_nonexistent_id_returns_error() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("goto", json!({"id": 999}));

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
            recent_logs: vec![LogLineInfo {
                id: 1,
                process: "web".to_string(),
                content: "Test log".to_string(),
                timestamp: "2025-12-17T10:00:00Z".to_string(),
                batch_id: None,
            }],
            total_log_lines: 1,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(!result.response.success);
        assert!(result.response.error.unwrap().contains("not found"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn goto_with_valid_id_returns_success_and_actions() {
        use super::super::state::{BufferStats, LogLineInfo, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::with_args("goto", json!({"id": 42}));

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
                    id: 42,
                    process: "web".to_string(),
                    content: "Target log line".to_string(),
                    timestamp: "2025-12-17T10:00:00Z".to_string(),
                    batch_id: None,
                },
                LogLineInfo {
                    id: 43,
                    process: "worker".to_string(),
                    content: "Another log".to_string(),
                    timestamp: "2025-12-17T10:00:01Z".to_string(),
                    batch_id: None,
                },
            ],
            total_log_lines: 2,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["scrolled_to"], 42);

        // Should emit ScrollToLine and SetAutoScroll actions
        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::ScrollToLine { id } if *id == 42
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn help_includes_goto_command() {
        let handler = test_handler();
        let request = IpcRequest::new("help");
        let result = handler.handle(&request, None);

        let data = result.response.result.unwrap();
        let commands = data["commands"].as_array().unwrap();

        let command_names: Vec<&str> = commands
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(command_names.contains(&"goto"));

        // Check that goto has the correct structure
        let goto_cmd = commands
            .iter()
            .find(|c| c["name"].as_str() == Some("goto"))
            .unwrap();

        assert!(goto_cmd["description"].as_str().unwrap().len() > 0);
        let args = goto_cmd["args"].as_array().unwrap();
        let arg_names: Vec<&str> = args
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        assert!(arg_names.contains(&"id"));
    }

    #[test]
    fn scroll_without_direction_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::new("scroll");
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.is_some());
        assert!(result.response.error.unwrap().contains("direction"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn scroll_with_invalid_direction_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::with_args("scroll", json!({"direction": "sideways"}));
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.unwrap().contains("invalid direction"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn scroll_up_returns_success_and_actions() {
        let handler = test_handler();
        let request = IpcRequest::with_args("scroll", json!({"direction": "up"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["direction"], "up");
        assert_eq!(data["lines"], 20); // default
        assert_eq!(data["auto_scroll"], false);

        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::ScrollUp { lines } if *lines == 20
        ));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn scroll_down_with_custom_lines() {
        let handler = test_handler();
        let request = IpcRequest::with_args("scroll", json!({"direction": "down", "lines": 50}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["direction"], "down");
        assert_eq!(data["lines"], 50);

        assert_eq!(result.actions.len(), 2);
        assert!(matches!(
            &result.actions[0],
            IpcAction::ScrollDown { lines } if *lines == 50
        ));
    }

    #[test]
    fn scroll_top_returns_scroll_to_top_action() {
        let handler = test_handler();
        let request = IpcRequest::with_args("scroll", json!({"direction": "top"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["direction"], "top");

        assert_eq!(result.actions.len(), 2);
        assert!(matches!(&result.actions[0], IpcAction::ScrollToTop));
        assert_eq!(result.actions[1], IpcAction::SetAutoScroll { enabled: false });
    }

    #[test]
    fn scroll_bottom_enables_auto_scroll() {
        let handler = test_handler();
        let request = IpcRequest::with_args("scroll", json!({"direction": "bottom"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["direction"], "bottom");
        assert_eq!(data["auto_scroll"], true);

        // Bottom only emits SetAutoScroll, not a separate scroll action
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0], IpcAction::SetAutoScroll { enabled: true });
    }

    #[test]
    fn help_includes_scroll_command() {
        let handler = test_handler();
        let request = IpcRequest::new("help");
        let result = handler.handle(&request, None);

        let data = result.response.result.unwrap();
        let commands = data["commands"].as_array().unwrap();

        let command_names: Vec<&str> = commands
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(command_names.contains(&"scroll"));

        // Check that scroll has the correct structure
        let scroll_cmd = commands
            .iter()
            .find(|c| c["name"].as_str() == Some("scroll"))
            .unwrap();

        assert!(scroll_cmd["description"].as_str().unwrap().len() > 0);
        let args = scroll_cmd["args"].as_array().unwrap();
        let arg_names: Vec<&str> = args
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        assert!(arg_names.contains(&"direction"));
        assert!(arg_names.contains(&"lines"));
    }

    #[test]
    fn freeze_toggle_default_without_state_freezes() {
        let handler = test_handler();
        let request = IpcRequest::new("freeze");
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["frozen"], true);
        assert_eq!(data["mode"], "toggle");

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetFrozen { frozen } if *frozen == true
        ));
    }

    #[test]
    fn freeze_toggle_with_unfrozen_state_freezes() {
        use super::super::state::{BufferStats, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("freeze");

        let snapshot = StateSnapshot {
            processes: vec![],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo {
                frozen: false,
                batch_view: false,
                trace_filter: false,
                trace_selection: false,
                compact: false,
            },
            auto_scroll: true,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
            recent_logs: Vec::new(),
            total_log_lines: 0,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["frozen"], true);

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetFrozen { frozen } if *frozen == true
        ));
    }

    #[test]
    fn freeze_toggle_with_frozen_state_unfreezes() {
        use super::super::state::{BufferStats, ViewModeInfo};

        let handler = test_handler();
        let request = IpcRequest::new("freeze");

        let snapshot = StateSnapshot {
            processes: vec![],
            filter_count: 0,
            active_filters: vec![],
            search_pattern: None,
            view_mode: ViewModeInfo {
                frozen: true,
                batch_view: false,
                trace_filter: false,
                trace_selection: false,
                compact: false,
            },
            auto_scroll: false,
            log_count: 0,
            buffer_stats: BufferStats::default(),
            trace_recording: false,
            active_trace_id: None,
            recent_logs: Vec::new(),
            total_log_lines: 0,
        };

        let result = handler.handle(&request, Some(&snapshot));

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["frozen"], false);

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetFrozen { frozen } if *frozen == false
        ));
    }

    #[test]
    fn freeze_on_explicitly_freezes() {
        let handler = test_handler();
        let request = IpcRequest::with_args("freeze", json!({"mode": "on"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["frozen"], true);
        assert_eq!(data["mode"], "on");

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetFrozen { frozen } if *frozen == true
        ));
    }

    #[test]
    fn freeze_off_explicitly_unfreezes() {
        let handler = test_handler();
        let request = IpcRequest::with_args("freeze", json!({"mode": "off"}));
        let result = handler.handle(&request, None);

        assert!(result.response.success);
        let data = result.response.result.unwrap();
        assert_eq!(data["frozen"], false);
        assert_eq!(data["mode"], "off");

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetFrozen { frozen } if *frozen == false
        ));
    }

    #[test]
    fn freeze_invalid_mode_returns_error() {
        let handler = test_handler();
        let request = IpcRequest::with_args("freeze", json!({"mode": "pause"}));
        let result = handler.handle(&request, None);

        assert!(!result.response.success);
        assert!(result.response.error.unwrap().contains("invalid mode"));
        assert!(result.actions.is_empty());
    }

    #[test]
    fn help_includes_freeze_command() {
        let handler = test_handler();
        let request = IpcRequest::new("help");
        let result = handler.handle(&request, None);

        let data = result.response.result.unwrap();
        let commands = data["commands"].as_array().unwrap();

        let command_names: Vec<&str> = commands
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(command_names.contains(&"freeze"));

        // Check that freeze has the correct structure
        let freeze_cmd = commands
            .iter()
            .find(|c| c["name"].as_str() == Some("freeze"))
            .unwrap();

        assert!(freeze_cmd["description"].as_str().unwrap().len() > 0);
        let args = freeze_cmd["args"].as_array().unwrap();
        let arg_names: Vec<&str> = args
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        assert!(arg_names.contains(&"mode"));
    }
}
