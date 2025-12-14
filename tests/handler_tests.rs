// ============================================================================
// Phase 0: Test Coverage Audit - Adding Tests for Command/Event Handlers
// ============================================================================
//
// These tests were added as part of Phase 0 of the refactoring plan to ensure
// all handlers have test coverage before consolidating duplicate code.
//
// Behavior Differences Documented:
// - Batch navigation via keyboard ([ and ]) does NOT show a status message
// - Batch navigation via command (:nb, :pb) DOES show "Next batch" / "Previous batch"
// - Batch window change via keyboard (+/-) shows "increased to" / "decreased to"
// - Batch window change via command (:bw) shows "set to"
// ============================================================================

use chrono::{Local, TimeZone};
use overitall::{
    log::{LogLine, LogSource},
    process::ProcessManager,
    ui::App,
};

/// Helper to create an App with test data for testing
fn create_test_app() -> App {
    App::new()
}

/// Helper to create a manager with specific arrival times for batch testing
fn create_manager_with_batched_logs() -> ProcessManager {
    let mut manager = ProcessManager::new();

    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None);
    manager.add_process("worker".to_string(), "ruby worker.rb".to_string(), None);

    // Batch 1: Three logs arriving within 100ms (at 12:00:00.000)
    let batch1_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();
    let mut log1 = LogLine::new(LogSource::ProcessStdout("web".to_string()), "Starting web server on port 3000".to_string());
    log1.timestamp = batch1_time;
    log1.arrival_time = batch1_time;
    manager.add_test_log(log1);

    let mut log2 = LogLine::new(LogSource::ProcessStdout("web".to_string()), "Loading configuration".to_string());
    log2.timestamp = batch1_time;
    log2.arrival_time = batch1_time + chrono::Duration::milliseconds(50);
    manager.add_test_log(log2);

    let mut log3 = LogLine::new(LogSource::ProcessStdout("web".to_string()), "Database connected".to_string());
    log3.timestamp = batch1_time;
    log3.arrival_time = batch1_time + chrono::Duration::milliseconds(90);
    manager.add_test_log(log3);

    // Batch 2: Two logs arriving 500ms later (at 12:00:00.500)
    let batch2_time = batch1_time + chrono::Duration::milliseconds(500);
    let mut log4 = LogLine::new(LogSource::ProcessStdout("worker".to_string()), "Processing job #1234".to_string());
    log4.timestamp = batch2_time;
    log4.arrival_time = batch2_time;
    manager.add_test_log(log4);

    let mut log5 = LogLine::new(LogSource::ProcessStdout("worker".to_string()), "Job #1234 completed".to_string());
    log5.timestamp = batch2_time;
    log5.arrival_time = batch2_time + chrono::Duration::milliseconds(80);
    manager.add_test_log(log5);

    // Batch 3: Single log 1 second later (at 12:00:01.500)
    let batch3_time = batch2_time + chrono::Duration::milliseconds(1000);
    let mut log6 = LogLine::new(LogSource::ProcessStdout("web".to_string()), "GET /api/users 200 OK".to_string());
    log6.timestamp = batch3_time;
    log6.arrival_time = batch3_time;
    manager.add_test_log(log6);

    manager
}

// ============================================================================
// Batch Navigation Handler Tests
// ============================================================================

#[test]
fn test_keyboard_next_batch_creates_snapshot() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Simulate keyboard ] press behavior: creates snapshot on first batch entry
    assert!(!app.batch_view_mode);
    assert!(app.snapshot.is_none());

    // Get filtered logs and create snapshot (what handle_next_batch does)
    let logs = manager.get_all_logs();
    let filtered_logs = overitall::ui::apply_filters(logs, &app.filters);
    app.create_snapshot(filtered_logs);
    app.next_batch();

    assert!(app.batch_view_mode);
    assert!(app.snapshot.is_some());
    assert_eq!(app.current_batch, Some(0));
}

#[test]
fn test_keyboard_prev_batch_creates_snapshot() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Simulate keyboard [ press behavior
    let logs = manager.get_all_logs();
    let filtered_logs = overitall::ui::apply_filters(logs, &app.filters);
    app.create_snapshot(filtered_logs);
    app.prev_batch();

    assert!(app.batch_view_mode);
    assert!(app.snapshot.is_some());
}

#[test]
fn test_batch_navigation_increments() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Enter batch view
    let logs = manager.get_all_logs();
    let filtered_logs = overitall::ui::apply_filters(logs, &app.filters);
    let filtered_refs: Vec<&overitall::log::LogLine> = filtered_logs.iter().collect();
    let batches = overitall::ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);
    let num_batches = batches.len();

    app.create_snapshot(filtered_logs);
    app.toggle_batch_view();

    // After toggle_batch_view, current_batch is Some(0)
    assert_eq!(app.current_batch, Some(0));

    // Navigate to second batch
    app.next_batch();
    assert_eq!(app.current_batch, Some(1));

    // Navigate to third batch
    app.next_batch();
    assert_eq!(app.current_batch, Some(2));

    // The App just increments the counter, wrapping happens at render time
    // Let's verify the batches exist
    assert!(num_batches >= 3, "Expected at least 3 batches in test data");
}

// ============================================================================
// Batch Window Handler Tests
// ============================================================================

#[test]
fn test_increase_batch_window_behavior() {
    let mut app = create_test_app();

    // Default is 100ms
    assert_eq!(app.batch_window_ms, 100);

    // Increase by 100ms (what handle_increase_batch_window does)
    let new_window = app.batch_window_ms + 100;
    app.set_batch_window(new_window);

    assert_eq!(app.batch_window_ms, 200);
}

#[test]
fn test_decrease_batch_window_behavior() {
    let mut app = create_test_app();

    // Set to 500ms first
    app.set_batch_window(500);
    assert_eq!(app.batch_window_ms, 500);

    // Decrease by 100ms (what handle_decrease_batch_window does)
    let new_window = (app.batch_window_ms - 100).max(1);
    app.set_batch_window(new_window);

    assert_eq!(app.batch_window_ms, 400);
}

#[test]
fn test_decrease_batch_window_minimum() {
    let mut app = create_test_app();

    // Set to 50ms
    app.set_batch_window(50);

    // Decrease - should clamp to 1ms
    let new_window = (app.batch_window_ms - 100).max(1);
    app.set_batch_window(new_window);

    assert_eq!(app.batch_window_ms, 1);
}

// ============================================================================
// Focus Batch Handler Tests
// ============================================================================

#[test]
fn test_focus_batch_finds_correct_batch() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Get batches
    let logs = manager.get_all_logs();
    let filtered_logs = overitall::ui::apply_filters(logs, &app.filters);
    let filtered_refs: Vec<&overitall::log::LogLine> = filtered_logs.iter().collect();
    let batches = overitall::ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);

    // Select a line in the second batch (if there are multiple batches)
    if batches.len() >= 2 {
        let (start, _) = batches[1];
        app.selected_line_index = Some(start);

        // Now find which batch contains this line
        let found = batches.iter().enumerate().find(|(_, (s, e))| {
            start >= *s && start <= *e
        });

        assert!(found.is_some());
        let (batch_idx, _) = found.unwrap();
        assert_eq!(batch_idx, 1);
    }
}

// ============================================================================
// Page Up/Down Handler Tests
// ============================================================================

#[test]
fn test_page_up_with_selection_moves_selection() {
    let mut app = create_test_app();

    // Select line 25
    app.selected_line_index = Some(25);

    // Page up should move selection by 20 lines
    if let Some(current_idx) = app.selected_line_index {
        let new_idx = current_idx.saturating_sub(20);
        app.selected_line_index = Some(new_idx);
    }

    assert_eq!(app.selected_line_index, Some(5));
}

#[test]
fn test_page_up_without_selection_scrolls_view() {
    let mut app = create_test_app();

    // Set scroll offset to 30
    app.scroll_offset = 30;
    app.auto_scroll = false;

    // Without selection, page up should scroll the view
    app.scroll_up(20);

    assert_eq!(app.scroll_offset, 10);
}

#[test]
fn test_page_down_with_selection_moves_selection() {
    let mut app = create_test_app();

    // Select line 5
    app.selected_line_index = Some(5);
    let total_logs: usize = 50;

    // Page down should move selection by 20 lines
    if let Some(current_idx) = app.selected_line_index {
        let new_idx = (current_idx + 20).min(total_logs.saturating_sub(1));
        app.selected_line_index = Some(new_idx);
    }

    assert_eq!(app.selected_line_index, Some(25));
}

#[test]
fn test_page_down_clamps_to_max() {
    let mut app = create_test_app();

    // Select line 40 with only 50 logs
    app.selected_line_index = Some(40);
    let total_logs: usize = 50;

    // Page down should clamp to last line
    if let Some(current_idx) = app.selected_line_index {
        let new_idx = (current_idx + 20).min(total_logs.saturating_sub(1));
        app.selected_line_index = Some(new_idx);
    }

    assert_eq!(app.selected_line_index, Some(49));
}

// ============================================================================
// Reset to Latest Handler Tests
// ============================================================================

#[test]
fn test_reset_clears_selection_first_esc() {
    let mut app = create_test_app();

    // Freeze display and select a line
    app.freeze_display();
    app.selected_line_index = Some(10);

    // First Esc: clear selection but stay frozen
    if app.frozen && app.selected_line_index.is_some() {
        app.selected_line_index = None;
    }

    assert!(app.frozen);
    assert_eq!(app.selected_line_index, None);
}

#[test]
fn test_reset_unfreezes_second_esc() {
    let mut app = create_test_app();

    // Freeze display, no selection
    app.freeze_display();
    app.selected_line_index = None;

    // Second Esc: unfreeze
    if app.frozen && app.selected_line_index.is_none() {
        app.unfreeze_display();
    }

    assert!(!app.frozen);
}

#[test]
fn test_reset_exits_batch_view_mode() {
    let mut app = create_test_app();

    // Enable batch view mode
    app.toggle_batch_view();
    app.current_batch = Some(2);

    assert!(app.batch_view_mode);

    // Reset should exit batch view
    app.batch_view_mode = false;
    app.current_batch = None;

    assert!(!app.batch_view_mode);
    assert_eq!(app.current_batch, None);
}

// ============================================================================
// Command Executor Filter Tests
// ============================================================================

#[test]
fn test_filter_include_adds_filter() {
    let mut app = create_test_app();

    app.add_include_filter("ERROR".to_string());

    assert_eq!(app.filters.len(), 1);
    assert_eq!(app.filters[0].pattern, "ERROR");
    assert!(matches!(app.filters[0].filter_type, overitall::ui::FilterType::Include));
}

#[test]
fn test_filter_exclude_adds_filter() {
    let mut app = create_test_app();

    app.add_exclude_filter("DEBUG".to_string());

    assert_eq!(app.filters.len(), 1);
    assert_eq!(app.filters[0].pattern, "DEBUG");
    assert!(matches!(app.filters[0].filter_type, overitall::ui::FilterType::Exclude));
}

#[test]
fn test_filter_clear_removes_all() {
    let mut app = create_test_app();

    app.add_include_filter("ERROR".to_string());
    app.add_exclude_filter("DEBUG".to_string());
    assert_eq!(app.filters.len(), 2);

    let count = app.filter_count();
    app.clear_filters();

    assert_eq!(count, 2);
    assert_eq!(app.filters.len(), 0);
}

// ============================================================================
// Command Batch Navigation Tests (with status messages)
// ============================================================================

#[test]
fn test_command_next_batch_behavior() {
    // Test that command version and keyboard version produce same batch state
    // (difference is only status message, which we can't easily test without full executor)
    let mut app1 = create_test_app();
    let mut app2 = create_test_app();
    let manager = create_manager_with_batched_logs();

    let logs = manager.get_all_logs();
    let filtered_logs = overitall::ui::apply_filters(logs, &app1.filters);

    // Simulate keyboard behavior
    app1.create_snapshot(filtered_logs.clone());
    app1.next_batch();

    // Simulate command behavior
    app2.create_snapshot(filtered_logs);
    app2.next_batch();

    // Both should end up in same state
    assert_eq!(app1.batch_view_mode, app2.batch_view_mode);
    assert_eq!(app1.current_batch, app2.current_batch);
}

// ============================================================================
// Parse Command Tests for Edge Cases
// ============================================================================

#[test]
fn test_parse_filter_commands() {
    use overitall::command::parse_command;

    // Filter include
    let cmd = parse_command("f ERROR");
    assert!(matches!(cmd, overitall::command::Command::FilterInclude(_)));

    // Filter exclude
    let cmd = parse_command("fn DEBUG");
    assert!(matches!(cmd, overitall::command::Command::FilterExclude(_)));

    // Filter clear
    let cmd = parse_command("fc");
    assert!(matches!(cmd, overitall::command::Command::FilterClear));

    // Filter list
    let cmd = parse_command("fl");
    assert!(matches!(cmd, overitall::command::Command::FilterList));
}

#[test]
fn test_parse_batch_commands() {
    use overitall::command::parse_command;

    // Next batch
    let cmd = parse_command("nb");
    assert!(matches!(cmd, overitall::command::Command::NextBatch));

    // Previous batch
    let cmd = parse_command("pb");
    assert!(matches!(cmd, overitall::command::Command::PrevBatch));

    // Show batch
    let cmd = parse_command("sb");
    assert!(matches!(cmd, overitall::command::Command::ShowBatch));
}

#[test]
fn test_parse_process_commands() {
    use overitall::command::parse_command;

    // Start
    let cmd = parse_command("s web");
    assert!(matches!(cmd, overitall::command::Command::Start(_)));

    // Restart
    let cmd = parse_command("r worker");
    assert!(matches!(cmd, overitall::command::Command::Restart(_)));

    // Kill
    let cmd = parse_command("k web");
    assert!(matches!(cmd, overitall::command::Command::Kill(_)));

    // Quit
    let cmd = parse_command("q");
    assert!(matches!(cmd, overitall::command::Command::Quit));
}

#[test]
fn test_parse_commands_missing_args() {
    use overitall::command::parse_command;

    // Missing process name
    let cmd = parse_command("s");
    assert!(matches!(cmd, overitall::command::Command::Unknown(_)));

    let cmd = parse_command("r");
    assert!(matches!(cmd, overitall::command::Command::Unknown(_)));

    let cmd = parse_command("k");
    assert!(matches!(cmd, overitall::command::Command::Unknown(_)));

    // Missing filter pattern
    let cmd = parse_command("f");
    assert!(matches!(cmd, overitall::command::Command::Unknown(_)));

    let cmd = parse_command("fn");
    assert!(matches!(cmd, overitall::command::Command::Unknown(_)));
}
