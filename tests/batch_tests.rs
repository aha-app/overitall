mod common;

use chrono::{Local, TimeZone};
use common::*;
use insta::assert_snapshot;
use overitall::log::{LogLine, LogSource};

#[test]
fn test_snapshot_batch_view_mode() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Enable batch view mode and select first batch
    app.batch.toggle_batch_view();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_batch_separators_visible() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Normal mode should show batch separators
    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_batch_navigation_second_batch() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Enable batch view and navigate to second batch
    app.batch.toggle_batch_view();
    app.batch.next_batch();
    app.navigation.scroll_offset = 0;
    app.navigation.auto_scroll = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_single_batch_no_separators() {
    let mut manager = overitall::process::ProcessManager::new();
    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None, None);

    // Add logs all in one batch (within 100ms)
    let batch_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();
    for i in 0..3 {
        let mut log = LogLine::new_with_time(
            LogSource::ProcessStdout("web".to_string()),
            format!("Log message {}", i + 1),
            batch_time,
        );
        log.arrival_time = batch_time + chrono::Duration::milliseconds(i * 30);
        manager.add_test_log(log);
    }

    let mut app = create_test_app();
    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}
