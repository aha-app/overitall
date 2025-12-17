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
    /// Scroll to a specific log line by ID (without opening expanded view)
    ScrollToLine { id: u64 },
    /// Scroll up by N lines
    ScrollUp { lines: usize },
    /// Scroll down by N lines
    ScrollDown { lines: usize },
    /// Scroll to the top of the log
    ScrollToTop,
    /// Set the frozen (paused) state of the display
    SetFrozen { frozen: bool },
    /// Add a filter (include or exclude)
    AddFilter { pattern: String, is_exclude: bool },
    /// Remove a filter by pattern
    RemoveFilter { pattern: String },
    /// Clear all filters
    ClearFilters,
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

    #[test]
    fn scroll_to_line_action_stores_id() {
        let action = IpcAction::ScrollToLine { id: 42 };
        match action {
            IpcAction::ScrollToLine { id } => assert_eq!(id, 42),
            _ => panic!("expected ScrollToLine"),
        }
    }

    #[test]
    fn scroll_to_line_action_equality() {
        let a1 = IpcAction::ScrollToLine { id: 42 };
        let a2 = IpcAction::ScrollToLine { id: 42 };
        let a3 = IpcAction::ScrollToLine { id: 99 };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn scroll_up_action_stores_lines() {
        let action = IpcAction::ScrollUp { lines: 20 };
        match action {
            IpcAction::ScrollUp { lines } => assert_eq!(lines, 20),
            _ => panic!("expected ScrollUp"),
        }
    }

    #[test]
    fn scroll_down_action_stores_lines() {
        let action = IpcAction::ScrollDown { lines: 50 };
        match action {
            IpcAction::ScrollDown { lines } => assert_eq!(lines, 50),
            _ => panic!("expected ScrollDown"),
        }
    }

    #[test]
    fn scroll_to_top_action_equality() {
        let a1 = IpcAction::ScrollToTop;
        let a2 = IpcAction::ScrollToTop;
        assert_eq!(a1, a2);
    }

    #[test]
    fn set_frozen_action_stores_state() {
        let action = IpcAction::SetFrozen { frozen: true };
        match action {
            IpcAction::SetFrozen { frozen } => assert!(frozen),
            _ => panic!("expected SetFrozen"),
        }
    }

    #[test]
    fn set_frozen_action_equality() {
        let a1 = IpcAction::SetFrozen { frozen: true };
        let a2 = IpcAction::SetFrozen { frozen: true };
        let a3 = IpcAction::SetFrozen { frozen: false };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn add_filter_action_stores_pattern_and_type() {
        let action = IpcAction::AddFilter {
            pattern: "error".to_string(),
            is_exclude: false,
        };
        match action {
            IpcAction::AddFilter { pattern, is_exclude } => {
                assert_eq!(pattern, "error");
                assert!(!is_exclude);
            }
            _ => panic!("expected AddFilter"),
        }
    }

    #[test]
    fn add_filter_action_equality() {
        let a1 = IpcAction::AddFilter {
            pattern: "error".to_string(),
            is_exclude: false,
        };
        let a2 = IpcAction::AddFilter {
            pattern: "error".to_string(),
            is_exclude: false,
        };
        let a3 = IpcAction::AddFilter {
            pattern: "error".to_string(),
            is_exclude: true,
        };
        let a4 = IpcAction::AddFilter {
            pattern: "warn".to_string(),
            is_exclude: false,
        };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
        assert_ne!(a1, a4);
    }

    #[test]
    fn remove_filter_action_stores_pattern() {
        let action = IpcAction::RemoveFilter {
            pattern: "debug".to_string(),
        };
        match action {
            IpcAction::RemoveFilter { pattern } => {
                assert_eq!(pattern, "debug");
            }
            _ => panic!("expected RemoveFilter"),
        }
    }

    #[test]
    fn remove_filter_action_equality() {
        let a1 = IpcAction::RemoveFilter {
            pattern: "error".to_string(),
        };
        let a2 = IpcAction::RemoveFilter {
            pattern: "error".to_string(),
        };
        let a3 = IpcAction::RemoveFilter {
            pattern: "warn".to_string(),
        };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn clear_filters_action_equality() {
        let a1 = IpcAction::ClearFilters;
        let a2 = IpcAction::ClearFilters;
        assert_eq!(a1, a2);
    }
}
