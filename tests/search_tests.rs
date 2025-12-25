mod common;

use common::*;
use insta::assert_snapshot;

#[test]
fn test_search_with_results() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Enter search mode and search for ERROR
    app.input.enter_search_mode();
    app.input.add_char('E');
    app.input.add_char('R');
    app.input.add_char('R');
    app.input.add_char('O');
    app.input.add_char('R');

    // Perform the search
    app.input.perform_search("ERROR".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // Should show search pattern in title or status
    assert!(output.contains("ERROR") || output.contains("Search"));

    assert_snapshot!(output);
}

#[test]
fn test_search_pattern_matching() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Search for a pattern that should match
    app.input.perform_search("job".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // The output should contain matched lines
    assert!(output.contains("job") || output.contains("Job"));
}

#[test]
fn test_search_as_filter() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Search for ERROR - this should filter logs to show only ERROR messages
    app.input.perform_search("ERROR".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert!(!output.is_empty());
    // Search pattern should be shown in the title
    assert!(output.contains("[Search: ERROR]"));
}

#[test]
fn test_search_with_filters() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Add a filter to include only web logs
    app.filters.add_include_filter("web".to_string());

    // Search for ERROR in filtered logs
    app.input.perform_search("ERROR".to_string());
    app.display.expanded_line_view = false;

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // Should show both filter and search
    assert!(output.contains("filter") || output.contains("1"));
}
