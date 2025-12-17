// Actions that IPC commands can trigger in the TUI
// These are returned alongside IpcResponse and processed by the main event loop

/// Actions that IPC command handlers can emit to update TUI state
#[derive(Debug, Clone, PartialEq)]
pub enum IpcAction {
    /// Set the search pattern and activate search highlighting
    SetSearch { pattern: String },
    /// Clear the current search
    ClearSearch,
    /// Enable or disable auto-scroll (tail mode)
    SetAutoScroll { enabled: bool },
    /// Select a log line by ID and open the expanded view
    SelectAndExpandLine { id: u64 },
}

/// Result of handling an IPC command: response to send + actions to apply
pub struct IpcHandlerResult {
    pub response: super::protocol::IpcResponse,
    pub actions: Vec<IpcAction>,
}

impl IpcHandlerResult {
    /// Create a result with just a response and no actions
    pub fn response_only(response: super::protocol::IpcResponse) -> Self {
        Self {
            response,
            actions: Vec::new(),
        }
    }

    /// Create a result with a response and actions
    pub fn with_actions(response: super::protocol::IpcResponse, actions: Vec<IpcAction>) -> Self {
        Self { response, actions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::IpcResponse;
    use serde_json::json;

    #[test]
    fn set_search_action_stores_pattern() {
        let action = IpcAction::SetSearch {
            pattern: "error".to_string(),
        };
        match action {
            IpcAction::SetSearch { pattern } => assert_eq!(pattern, "error"),
            _ => panic!("expected SetSearch"),
        }
    }

    #[test]
    fn handler_result_response_only_has_empty_actions() {
        let result = IpcHandlerResult::response_only(IpcResponse::ok(json!({"ok": true})));
        assert!(result.actions.is_empty());
        assert!(result.response.success);
    }

    #[test]
    fn handler_result_with_actions_stores_actions() {
        let actions = vec![IpcAction::SetSearch {
            pattern: "test".to_string(),
        }];
        let result =
            IpcHandlerResult::with_actions(IpcResponse::ok(json!({"matches": []})), actions);

        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            IpcAction::SetSearch { pattern } if pattern == "test"
        ));
    }

    #[test]
    fn clear_search_action_equality() {
        let a1 = IpcAction::ClearSearch;
        let a2 = IpcAction::ClearSearch;
        assert_eq!(a1, a2);
    }

    #[test]
    fn set_search_action_equality() {
        let a1 = IpcAction::SetSearch {
            pattern: "foo".to_string(),
        };
        let a2 = IpcAction::SetSearch {
            pattern: "foo".to_string(),
        };
        let a3 = IpcAction::SetSearch {
            pattern: "bar".to_string(),
        };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }
}
