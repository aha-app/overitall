mod common;
use common::*;
use insta::assert_snapshot;

use chrono::{Local, TimeZone};
use overitall::{
    log::{LogLine, LogSource},
    process::ProcessManager,
};

#[test]
fn test_batch_window_default_value() {
    let app = create_test_app();

    // Default batch window should be 100ms
    assert_eq!(app.batch.batch_window_ms, 100);
}

#[test]
fn test_batch_window_set_value() {
    let mut app = create_test_app();

    // Change batch window to 500ms
    app.batch.set_batch_window(500);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    assert_eq!(app.batch.batch_window_ms, 500);
}

#[test]
fn test_batch_window_affects_batch_detection() {
    let mut manager = ProcessManager::new();
    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);

    // Create logs with 200ms gap between them
    let base_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

    let log1 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "Log 1".to_string(), base_time);
    manager.add_test_log(log1);

    let mut log2 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "Log 2".to_string(), base_time);
    log2.arrival_time = base_time + chrono::Duration::milliseconds(200);
    manager.add_test_log(log2);

    // With 100ms window, should be 2 batches
    let logs = manager.get_all_logs();
    let batches_100ms = overitall::ui::detect_batches_from_logs(&logs, 100);
    assert_eq!(batches_100ms.len(), 2, "With 100ms window, should have 2 batches");

    // With 300ms window, should be 1 batch
    let batches_300ms = overitall::ui::detect_batches_from_logs(&logs, 300);
    assert_eq!(batches_300ms.len(), 1, "With 300ms window, should have 1 batch");
}

#[test]
fn test_batch_window_snapshot_with_small_window() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Set very small batch window (50ms)
    // This should create more batches from the test data
    app.batch.set_batch_window(50);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_batch_window_snapshot_with_large_window() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Set large batch window (5000ms = 5 seconds)
    // This should group all test logs into a single batch
    app.batch.set_batch_window(5000);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_batch_window_resets_batch_view_if_active() {
    let mut app = create_test_app();

    // Enable batch view and select batch 2
    app.batch.toggle_batch_view();
    app.batch.next_batch();
    app.navigation.scroll_offset = 0;
    app.navigation.auto_scroll = false;
    app.batch.current_batch = Some(1); // Manually set to batch 2

    // Changing batch window should reset to batch 0
    app.batch.set_batch_window(500);
    if app.batch.batch_view_mode {
        app.navigation.scroll_offset = 0;
    }

    assert_eq!(app.batch.current_batch, Some(0), "Should reset to first batch");
}

#[test]
fn test_batch_window_prevents_chaining() {
    // Regression test for the "chaining" bug where logs slowly drift apart
    // over time but each consecutive pair is within the window
    let mut manager = ProcessManager::new();
    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);

    let base_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

    // Create logs with 2-second gaps between each
    // Log 1 at 0s, Log 2 at 2s, Log 3 at 4s, Log 4 at 6s
    for i in 0..4 {
        let mut log = LogLine::new_with_time(
            LogSource::ProcessStdout("web".to_string()),
            format!("Log {}", i + 1),
            base_time,
        );
        log.arrival_time = base_time + chrono::Duration::seconds(i * 2);
        manager.add_test_log(log);
    }

    let logs = manager.get_all_logs();

    // With a 3000ms (3 second) window:
    // - Log 1 at 0s (batch starts at 0s)
    // - Log 2 at 2s (2s from batch start, < 3s window, SAME batch)
    // - Log 3 at 4s (4s from batch start, > 3s window, NEW batch)
    // - Log 4 at 6s (2s from new batch start at 4s, < 3s window, SAME batch)
    //
    // Expected: 2 batches
    // - Batch 1: indices 0-1 (logs 1-2)
    // - Batch 2: indices 2-3 (logs 3-4)
    let batches = overitall::ui::detect_batches_from_logs(&logs, 3000);

    assert_eq!(batches.len(), 2, "Should have 2 batches with 3s window");
    assert_eq!(batches[0], (0, 1), "First batch should contain logs 0-1");
    assert_eq!(batches[1], (2, 3), "Second batch should contain logs 2-3");

    // With the OLD buggy algorithm (comparing to previous log):
    // All logs would be in one batch because each consecutive pair is 2s < 3s
    // This test ensures we're comparing to the batch START, not previous log
}
