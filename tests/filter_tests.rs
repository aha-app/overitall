mod common;

use common::*;
use insta::assert_snapshot;

// --- Filtering Snapshot Tests ---

#[test]
fn test_snapshot_include_filter_active() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Apply include filter for "ERROR"
    app.filters.add_include_filter("ERROR".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_exclude_filter_active() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Apply exclude filter for "ERROR"
    app.filters.add_exclude_filter("ERROR".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_multiple_filters_active() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Apply both include and exclude filters
    app.filters.add_include_filter("job".to_string());
    app.filters.add_exclude_filter("ERROR".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_filter_list_display() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Add multiple filters
    app.filters.add_include_filter("ERROR".to_string());
    app.filters.add_include_filter("web".to_string());
    app.filters.add_exclude_filter("DEBUG".to_string());

    // Enter command mode to show filter list command
    app.input.enter_command_mode();
    app.input.add_char('f');
    app.input.add_char('l');

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_empty_results_after_filtering() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Apply filter that matches nothing
    app.filters.add_include_filter("NONEXISTENT_PATTERN_XYZ".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}
