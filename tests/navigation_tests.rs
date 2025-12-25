mod common;

use common::*;
use insta::assert_snapshot;

#[test]
fn test_wraparound_top_to_bottom() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Use operations module for navigation
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    // Now at first line, press "Up" to wrap to bottom
    overitall::operations::navigation::select_prev_line(&mut app, &manager);

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_wraparound_bottom_to_top() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Navigate to last line by selecting down multiple times
    for _ in 0..8 {
        overitall::operations::navigation::select_next_line(&mut app, &manager);
    }

    // Now at bottom, press "Down" to wrap to top
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_wraparound_in_batch_view() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Enable batch view mode
    app.batch.toggle_batch_view();

    // Select first line in batch
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    // Wrap from top to bottom within batch
    overitall::operations::navigation::select_prev_line(&mut app, &manager);

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_wraparound_with_filters() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Apply a filter to reduce visible logs
    app.filters.add_include_filter("ERROR".to_string());
    // This should leave 2 logs visible

    // Select first filtered line
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    // Wrap from top to bottom
    overitall::operations::navigation::select_prev_line(&mut app, &manager);

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}
