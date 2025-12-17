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

    pub fn handle(&self, request: &IpcRequest, _state: Option<&StateSnapshot>) -> IpcResponse {
        match request.command.as_str() {
            "ping" => self.handle_ping(),
            "status" => self.handle_status(&request.args),
            _ => IpcResponse::err(format!("unknown command: {}", request.command)),
        }
    }

    fn handle_ping(&self) -> IpcResponse {
        IpcResponse::ok(json!({"pong": true}))
    }

    fn handle_status(&self, _args: &Value) -> IpcResponse {
        IpcResponse::ok(json!({
            "version": self.version,
            "running": true
        }))
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
}
