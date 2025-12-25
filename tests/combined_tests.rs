mod common;

use common::*;
use insta::assert_snapshot;

#[test]
fn test_snapshot_filtering_and_batching_combined() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Apply filter and enable batch view
    app.filters.add_include_filter("web".to_string());
    app.batch.toggle_batch_view();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_search_and_filtering_combined() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Add filter
    app.filters.add_include_filter("ERROR".to_string());

    // Perform search
    app.input.perform_search("Database".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_search_and_batching_combined() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Enable batch view
    app.batch.toggle_batch_view();

    // Perform search
    app.input.perform_search("job".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_all_features_active() {
    let mut app = create_test_app();
    let manager = create_manager_with_batched_logs();

    // Apply filter
    app.filters.add_include_filter("web".to_string());

    // Enable batch view
    app.batch.toggle_batch_view();

    // Perform search
    app.input.perform_search("server".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}
