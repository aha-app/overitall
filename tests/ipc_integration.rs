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
