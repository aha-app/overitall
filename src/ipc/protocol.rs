use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request message sent from CLI client to TUI server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcRequest {
    /// Command name (e.g., "ping", "status", "search")
    pub command: String,
    /// Command arguments as JSON value
    #[serde(default)]
    pub args: Value,
}

impl IpcRequest {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Value::Null,
        }
    }

    pub fn with_args(command: impl Into<String>, args: Value) -> Self {
        Self {
            command: command.into(),
            args,
        }
    }
}

/// Response message sent from TUI server to CLI client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcResponse {
    /// Whether the command succeeded
    pub success: bool,
    /// Result data on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IpcResponse {
    pub fn ok(result: Value) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn ok_empty() -> Self {
        Self {
            success: true,
            result: None,
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_new_creates_simple_request() {
        let req = IpcRequest::new("ping");
        assert_eq!(req.command, "ping");
        assert_eq!(req.args, Value::Null);
    }

    #[test]
    fn request_with_args_creates_request_with_data() {
        let req = IpcRequest::with_args("search", json!({"pattern": "error", "limit": 10}));
        assert_eq!(req.command, "search");
        assert_eq!(req.args["pattern"], "error");
        assert_eq!(req.args["limit"], 10);
    }

    #[test]
    fn request_serialization_roundtrip() {
        let req = IpcRequest::with_args("search", json!({"pattern": "test"}));
        let json = serde_json::to_string(&req).unwrap();
        let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, parsed);
    }

    #[test]
    fn request_deserialize_without_args() {
        let json = r#"{"command": "ping"}"#;
        let req: IpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.command, "ping");
        assert_eq!(req.args, Value::Null);
    }

    #[test]
    fn response_ok_creates_success_response() {
        let resp = IpcResponse::ok(json!({"status": "running"}));
        assert!(resp.success);
        assert_eq!(resp.result, Some(json!({"status": "running"})));
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_ok_empty_creates_success_without_result() {
        let resp = IpcResponse::ok_empty();
        assert!(resp.success);
        assert!(resp.result.is_none());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_err_creates_error_response() {
        let resp = IpcResponse::err("command not found");
        assert!(!resp.success);
        assert!(resp.result.is_none());
        assert_eq!(resp.error, Some("command not found".to_string()));
    }

    #[test]
    fn response_serialization_roundtrip_success() {
        let resp = IpcResponse::ok(json!({"count": 42}));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    #[test]
    fn response_serialization_roundtrip_error() {
        let resp = IpcResponse::err("something went wrong");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    #[test]
    fn response_json_skips_none_fields() {
        let resp = IpcResponse::ok_empty();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("result"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn error_response_skips_result_field() {
        let resp = IpcResponse::err("error");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("result"));
        assert!(json.contains("error"));
    }

    #[test]
    fn success_response_skips_error_field() {
        let resp = IpcResponse::ok(json!("data"));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("result"));
        assert!(!json.contains("error"));
    }
}
