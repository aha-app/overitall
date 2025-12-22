use std::path::PathBuf;
use std::time::Duration;

use overitall::ipc::{IpcClient, IpcCommandHandler, IpcRequest, IpcServer};
use serde_json::json;
use tempfile::TempDir;

fn temp_socket_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sock");
    (dir, path)
}

/// Full ping flow: client sends ping -> server receives -> handler processes -> response sent -> client receives
#[tokio::test]
async fn test_ping_integration() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0-test");

    // Connect client
    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Client sends ping request
    let request = IpcRequest::new("ping");
    client.send_request(&request).await.unwrap();

    // Server receives and processes via handler
    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "ping");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Server sends response back
    server.send_response(conn_id, handler_result.response).await.unwrap();

    // Client receives response
    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    assert_eq!(received.result, Some(json!({"pong": true})));
}

/// Status returns version from handler
#[tokio::test]
async fn test_status_returns_version() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("1.2.3");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send status request
    let request = IpcRequest::new("status");
    client.send_request(&request).await.unwrap();

    // Process on server side
    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);
    server.send_response(conn_id, handler_result.response).await.unwrap();

    // Verify response
    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();
    assert_eq!(result["version"], "1.2.3");
    assert_eq!(result["running"], true);
}

/// Multiple clients can connect and communicate
#[tokio::test]
async fn test_multiple_clients() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    // Connect three clients
    let mut client1 = IpcClient::connect(&path).await.unwrap();
    let mut client2 = IpcClient::connect(&path).await.unwrap();
    let mut client3 = IpcClient::connect(&path).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let accepted = server.accept_pending().unwrap();
    assert_eq!(accepted, 3);
    assert_eq!(server.connection_count(), 3);

    // All clients send ping
    client1.send_request(&IpcRequest::new("ping")).await.unwrap();
    client2.send_request(&IpcRequest::new("ping")).await.unwrap();
    client3.send_request(&IpcRequest::new("ping")).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Server receives all requests
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 3);

    // Process and send responses to all
    for (conn_id, req) in &requests {
        let handler_result = handler.handle(req, None);
        server.send_response(*conn_id, handler_result.response).await.unwrap();
    }

    // All clients receive responses
    let resp1 = client1.recv_response().await.unwrap();
    let resp2 = client2.recv_response().await.unwrap();
    let resp3 = client3.recv_response().await.unwrap();

    assert!(resp1.success);
    assert!(resp2.success);
    assert!(resp3.success);
    assert_eq!(resp1.result, Some(json!({"pong": true})));
    assert_eq!(resp2.result, Some(json!({"pong": true})));
    assert_eq!(resp3.result, Some(json!({"pong": true})));
}

/// Socket file is cleaned up when server drops
#[tokio::test]
async fn test_socket_cleanup() {
    let (_dir, path) = temp_socket_path();

    // Create and verify socket exists
    {
        let _server = IpcServer::new(&path).unwrap();
        assert!(path.exists(), "socket should exist while server is alive");
    }
    // Server dropped here

    assert!(!path.exists(), "socket should be cleaned up after server drops");
}

/// Socket can be reused after cleanup
#[tokio::test]
async fn test_socket_reuse_after_cleanup() {
    let (_dir, path) = temp_socket_path();

    // First server
    {
        let _server = IpcServer::new(&path).unwrap();
        assert!(path.exists());
    }

    // Second server at same path
    {
        let _server = IpcServer::new(&path).unwrap();
        assert!(path.exists());
    }
}

/// Unknown commands return error response
#[tokio::test]
async fn test_unknown_command_returns_error() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send unknown command
    let request = IpcRequest::new("nonexistent_command");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(!received.success);
    assert_eq!(
        received.error,
        Some("unknown command: nonexistent_command".to_string())
    );
}

/// Request with arguments works end-to-end
#[tokio::test]
async fn test_request_with_args() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send status with args (handler ignores them, but ensure they pass through)
    let request = IpcRequest::with_args("status", json!({"verbose": true, "format": "json"}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.args["verbose"], true);
    assert_eq!(requests[0].1.args["format"], "json");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
}

/// Client disconnect is handled gracefully
#[tokio::test]
async fn test_client_disconnect_handling() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();

    // Connect and then disconnect
    {
        let _client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        server.accept_pending().unwrap();
        assert_eq!(server.connection_count(), 1);
    }
    // Client dropped

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Server detects disconnect on next poll
    let _ = server.poll_commands().unwrap();
    assert_eq!(server.connection_count(), 0);
}

/// Rapid connect/disconnect doesn't cause issues
#[tokio::test]
async fn test_rapid_connect_disconnect() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();

    for _ in 0..10 {
        let _client = IpcClient::connect(&path).await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        server.accept_pending().unwrap();
    }

    // Give time for disconnects to propagate
    tokio::time::sleep(Duration::from_millis(20)).await;
    let _ = server.poll_commands().unwrap();

    // All clients have disconnected
    assert_eq!(server.connection_count(), 0);
}

// ============================================================================
// Process Control Integration Tests
// ============================================================================

/// Restart command with specific process name returns success and action
#[tokio::test]
async fn test_restart_specific_process() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send restart command for specific process
    let request = IpcRequest::with_args("restart", json!({"name": "web"}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "restart");
    assert_eq!(requests[0].1.args["name"], "web");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify handler returns restart action
    assert_eq!(handler_result.actions.len(), 1);

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();
    assert_eq!(result["restarting"], true);
    assert_eq!(result["process"], "web");
}

/// Restart command without process name restarts all
#[tokio::test]
async fn test_restart_all_processes() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send restart command without name (restart all)
    let request = IpcRequest::new("restart");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "restart");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify handler returns restart all action
    assert_eq!(handler_result.actions.len(), 1);

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();
    assert_eq!(result["restarting"], true);
    assert_eq!(result["process"], "all");
}

/// Kill command with process name returns success
#[tokio::test]
async fn test_kill_process() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send kill command
    let request = IpcRequest::with_args("kill", json!({"name": "worker"}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "kill");
    assert_eq!(requests[0].1.args["name"], "worker");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify handler returns kill action
    assert_eq!(handler_result.actions.len(), 1);

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();
    assert_eq!(result["killed"], true);
    assert_eq!(result["name"], "worker");
}

/// Kill command without name returns error
#[tokio::test]
async fn test_kill_without_name_returns_error() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send kill command without name
    let request = IpcRequest::new("kill");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify no actions on error
    assert!(handler_result.actions.is_empty());

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(!received.success);
    assert!(received.error.is_some());
    assert!(received.error.unwrap().contains("name"));
}

/// Start command with process name returns success
#[tokio::test]
async fn test_start_process() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send start command
    let request = IpcRequest::with_args("start", json!({"name": "api"}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "start");
    assert_eq!(requests[0].1.args["name"], "api");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify handler returns start action
    assert_eq!(handler_result.actions.len(), 1);

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();
    assert_eq!(result["started"], true);
    assert_eq!(result["name"], "api");
}

/// Start command without name returns error
#[tokio::test]
async fn test_start_without_name_returns_error() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Send start command without name
    let request = IpcRequest::new("start");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, None);

    // Verify no actions on error
    assert!(handler_result.actions.is_empty());

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(!received.success);
    assert!(received.error.is_some());
    assert!(received.error.unwrap().contains("name"));
}

// ============================================================================
// AI-Optimized Commands Integration Tests
// ============================================================================

use overitall::ipc::state::{BufferStats, FilterInfo, LogLineInfo, ProcessInfo, StateSnapshot, ViewModeInfo};

/// Helper to create a test snapshot with log lines
fn test_snapshot_with_logs(logs: Vec<LogLineInfo>) -> StateSnapshot {
    StateSnapshot {
        processes: vec![
            ProcessInfo {
                name: "web".to_string(),
                status: "running".to_string(),
                error: None,
                custom_label: None,
                custom_color: None,
            },
            ProcessInfo {
                name: "worker".to_string(),
                status: "running".to_string(),
                error: None,
                custom_label: None,
                custom_color: None,
            },
        ],
        log_files: vec![],
        filter_count: 0,
        active_filters: vec![],
        search_pattern: None,
        view_mode: ViewModeInfo::default(),
        auto_scroll: true,
        log_count: logs.len(),
        buffer_stats: BufferStats::default(),
        trace_recording: false,
        active_trace_id: None,
        total_log_lines: logs.len(),
        hidden_processes: vec![],
        recent_logs: logs,
    }
}

/// Errors command returns error logs with level detection
#[tokio::test]
async fn test_errors_command_returns_error_logs() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Create snapshot with mixed logs
    let snapshot = test_snapshot_with_logs(vec![
        LogLineInfo {
            id: 1,
            process: "web".to_string(),
            content: "Server started on port 3000".to_string(),
            timestamp: "2025-12-17T10:00:00Z".to_string(),
            batch_id: None,
        },
        LogLineInfo {
            id: 2,
            process: "web".to_string(),
            content: "Error: connection refused".to_string(),
            timestamp: "2025-12-17T10:00:01Z".to_string(),
            batch_id: None,
        },
        LogLineInfo {
            id: 3,
            process: "worker".to_string(),
            content: "Warning: slow query detected".to_string(),
            timestamp: "2025-12-17T10:00:02Z".to_string(),
            batch_id: None,
        },
        LogLineInfo {
            id: 4,
            process: "worker".to_string(),
            content: "Job failed with exit code 1".to_string(),
            timestamp: "2025-12-17T10:00:03Z".to_string(),
            batch_id: None,
        },
    ]);

    // Send errors command
    let request = IpcRequest::new("errors");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1.command, "errors");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();

    // Should find 2 error logs (not warnings by default)
    let errors = result["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    assert_eq!(result["level_filter"], "error");

    // Results are newest first
    assert_eq!(errors[0]["id"], 4);
    assert_eq!(errors[0]["level"], "error");
    assert_eq!(errors[1]["id"], 2);
}

/// Errors command with level filter returns both errors and warnings
#[tokio::test]
async fn test_errors_command_with_level_filter() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    let snapshot = test_snapshot_with_logs(vec![
        LogLineInfo {
            id: 1,
            process: "web".to_string(),
            content: "Error: database connection lost".to_string(),
            timestamp: "2025-12-17T10:00:00Z".to_string(),
            batch_id: None,
        },
        LogLineInfo {
            id: 2,
            process: "worker".to_string(),
            content: "Warning: memory usage high".to_string(),
            timestamp: "2025-12-17T10:00:01Z".to_string(),
            batch_id: None,
        },
    ]);

    // Send errors command with error_or_warning level
    let request = IpcRequest::with_args("errors", json!({"level": "error_or_warning"}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();

    // Should find both error and warning
    let errors = result["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    assert_eq!(result["level_filter"], "error_or_warning");
}

/// Summary command returns comprehensive state overview
#[tokio::test]
async fn test_summary_command_returns_comprehensive_state() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("1.0.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Create a rich snapshot with various state
    let snapshot = StateSnapshot {
        processes: vec![
            ProcessInfo {
                name: "web".to_string(),
                status: "running".to_string(),
                error: None,
                custom_label: None,
                custom_color: None,
            },
            ProcessInfo {
                name: "worker".to_string(),
                status: "failed".to_string(),
                error: Some("Exit code: 1".to_string()),
                custom_label: None,
                custom_color: None,
            },
            ProcessInfo {
                name: "scheduler".to_string(),
                status: "stopped".to_string(),
                error: None,
                custom_label: None,
                custom_color: None,
            },
        ],
        log_files: vec![],
        filter_count: 2,
        active_filters: vec![
            FilterInfo {
                pattern: "debug".to_string(),
                filter_type: "exclude".to_string(),
            },
            FilterInfo {
                pattern: "error".to_string(),
                filter_type: "include".to_string(),
            },
        ],
        search_pattern: None,
        view_mode: ViewModeInfo {
            frozen: true,
            batch_view: false,
            trace_filter: false,
            trace_selection: false,
            display_mode: "compact".to_string(),
        },
        auto_scroll: false,
        log_count: 500,
        buffer_stats: BufferStats {
            buffer_bytes: 10000000,
            max_buffer_bytes: 52428800,
            usage_percent: 19.07,
        },
        trace_recording: true,
        active_trace_id: Some("trace123".to_string()),
        recent_logs: vec![
            LogLineInfo {
                id: 1,
                process: "web".to_string(),
                content: "Error: connection timeout".to_string(),
                timestamp: "2025-12-17T10:00:00Z".to_string(),
                batch_id: None,
            },
        ],
        total_log_lines: 500,
        hidden_processes: vec!["scheduler".to_string()],
    };

    // Send summary command
    let request = IpcRequest::new("summary");
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests[0].1.command, "summary");

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();

    // Verify status section
    assert_eq!(result["status"]["version"], "1.0.0");
    assert_eq!(result["status"]["running"], true);

    // Verify processes section
    assert_eq!(result["processes"]["total"], 3);
    assert_eq!(result["processes"]["running"], 1);
    assert_eq!(result["processes"]["stopped"], 1);
    assert_eq!(result["processes"]["failed"], 1);
    let details = result["processes"]["details"].as_array().unwrap();
    assert_eq!(details.len(), 3);

    // Verify logs section
    assert_eq!(result["logs"]["total_lines"], 500);
    assert_eq!(result["logs"]["buffer_bytes"], 10000000);

    // Verify errors section
    assert_eq!(result["errors"]["recent_count"], 1);
    assert!(result["errors"]["last_error"].is_object());

    // Verify filters section
    assert_eq!(result["filters"]["count"], 2);
    let active = result["filters"]["active"].as_array().unwrap();
    assert_eq!(active.len(), 2);

    // Verify view section
    assert_eq!(result["view"]["frozen"], true);
    assert_eq!(result["view"]["auto_scroll"], false);
    assert_eq!(result["view"]["display_mode"], "compact");
    assert_eq!(result["view"]["trace_recording"], true);
}

/// Batch command returns all lines from a specific batch
#[tokio::test]
async fn test_batch_command_returns_batch_lines() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    // Create snapshot with logs in different batches
    let snapshot = test_snapshot_with_logs(vec![
        LogLineInfo {
            id: 1,
            process: "web".to_string(),
            content: "Request started".to_string(),
            timestamp: "2025-12-17T10:00:00Z".to_string(),
            batch_id: Some(10),
        },
        LogLineInfo {
            id: 2,
            process: "web".to_string(),
            content: "Processing request".to_string(),
            timestamp: "2025-12-17T10:00:01Z".to_string(),
            batch_id: Some(10),
        },
        LogLineInfo {
            id: 3,
            process: "web".to_string(),
            content: "Request completed".to_string(),
            timestamp: "2025-12-17T10:00:02Z".to_string(),
            batch_id: Some(10),
        },
        LogLineInfo {
            id: 4,
            process: "worker".to_string(),
            content: "Different batch".to_string(),
            timestamp: "2025-12-17T10:00:03Z".to_string(),
            batch_id: Some(11),
        },
    ]);

    // Send batch command for batch 10
    let request = IpcRequest::with_args("batch", json!({"id": 10}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();
    assert_eq!(requests[0].1.command, "batch");
    assert_eq!(requests[0].1.args["id"], 10);

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));

    // No actions emitted without scroll flag
    assert!(handler_result.actions.is_empty());

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();

    assert_eq!(result["batch_id"], 10);
    assert_eq!(result["count"], 3);
    assert_eq!(result["process"], "web");

    let lines = result["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[1]["id"], 2);
    assert_eq!(lines[2]["id"], 3);
}

/// Batch command with scroll flag emits scroll action
#[tokio::test]
async fn test_batch_command_with_scroll_flag() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    let snapshot = test_snapshot_with_logs(vec![
        LogLineInfo {
            id: 42,
            process: "web".to_string(),
            content: "Batch line 1".to_string(),
            timestamp: "2025-12-17T10:00:00Z".to_string(),
            batch_id: Some(7),
        },
        LogLineInfo {
            id: 43,
            process: "web".to_string(),
            content: "Batch line 2".to_string(),
            timestamp: "2025-12-17T10:00:01Z".to_string(),
            batch_id: Some(7),
        },
    ]);

    // Send batch command with scroll flag
    let request = IpcRequest::with_args("batch", json!({"id": 7, "scroll": true}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));

    // Should emit scroll actions
    assert_eq!(handler_result.actions.len(), 2);

    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(received.success);
    let result = received.result.unwrap();

    assert_eq!(result["batch_id"], 7);
    assert_eq!(result["count"], 2);
}

/// Batch command with nonexistent batch ID returns error
#[tokio::test]
async fn test_batch_command_nonexistent_batch_returns_error() {
    let (_dir, path) = temp_socket_path();
    let mut server = IpcServer::new(&path).unwrap();
    let handler = IpcCommandHandler::new("0.1.0");

    let mut client = IpcClient::connect(&path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    server.accept_pending().unwrap();

    let snapshot = test_snapshot_with_logs(vec![
        LogLineInfo {
            id: 1,
            process: "web".to_string(),
            content: "Some log".to_string(),
            timestamp: "2025-12-17T10:00:00Z".to_string(),
            batch_id: Some(5),
        },
    ]);

    // Request a batch that doesn't exist
    let request = IpcRequest::with_args("batch", json!({"id": 999}));
    client.send_request(&request).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;
    let requests = server.poll_commands().unwrap();

    let conn_id = requests[0].0;
    let handler_result = handler.handle(&requests[0].1, Some(&snapshot));
    server.send_response(conn_id, handler_result.response).await.unwrap();

    let received = client.recv_response().await.unwrap();
    assert!(!received.success);
    assert!(received.error.unwrap().contains("not found"));
}
