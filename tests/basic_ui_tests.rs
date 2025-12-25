mod common;

use common::{create_manager_with_logs, create_test_app, create_test_process_manager, render_app_to_string};
use insta::assert_snapshot;

#[test]
fn test_basic_ui_rendering() {
    let mut app = create_test_app();
    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!("ui_tests__basic_ui_rendering", output);
}

#[test]
fn test_search_mode_display() {
    let mut app = create_test_app();
    app.input.enter_search_mode();
    app.input.add_char('E');
    app.input.add_char('R');
    app.input.add_char('R');
    app.input.add_char('O');
    app.input.add_char('R');

    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!("ui_tests__search_mode_display", output);
}

#[test]
fn test_command_mode_display() {
    let mut app = create_test_app();
    app.input.enter_command_mode();
    app.input.add_char('r');
    app.input.add_char(' ');
    app.input.add_char('w');
    app.input.add_char('e');
    app.input.add_char('b');

    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!("ui_tests__command_mode_display", output);
}


#[test]
fn test_filter_display() {
    let mut app = create_test_app();
    app.filters.add_include_filter("ERROR".to_string());
    app.filters.add_exclude_filter("DEBUG".to_string());

    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert!(output.contains("2"));
    assert!(output.contains("filter"));
}

#[test]
fn test_status_message_success() {
    let mut app = create_test_app();
    app.display.set_status_success("Process restarted successfully".to_string());

    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert!(!output.is_empty());
}

#[test]
fn test_status_message_error() {
    let mut app = create_test_app();
    app.display.set_status_error("Failed to restart process".to_string());

    let manager = create_test_process_manager();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert!(!output.is_empty());
}

#[test]
fn test_log_display_with_data() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    assert!(output.contains("Starting web server"));
    assert!(output.contains("Processing job"));
    assert!(output.contains("ERROR"));

    assert!(output.contains("web"));
    assert!(output.contains("worker"));
}

#[test]
fn test_log_formatting() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    assert!(output.contains(":"));
}

#[test]
fn test_empty_search_pattern() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    app.input.enter_search_mode();

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    assert!(!output.is_empty());
}
