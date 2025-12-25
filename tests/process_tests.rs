mod common;

use common::*;
use insta::assert_snapshot;

// ============================================================================
// Process Visibility Toggle Tests
// ============================================================================

#[test]
fn test_hide_command_parsing() {
    use overitall::command::parse_command;

    let cmd = parse_command("hide worker");
    assert!(matches!(cmd, overitall::command::Command::Hide(_)));
}

#[test]
fn test_show_command_parsing() {
    use overitall::command::parse_command;

    let cmd = parse_command("show worker");
    assert!(matches!(cmd, overitall::command::Command::Show(_)));
}

#[test]
fn test_hide_all_command_parsing() {
    use overitall::command::parse_command;

    let cmd = parse_command("hide all");
    assert!(matches!(cmd, overitall::command::Command::HideAll));
}

#[test]
fn test_show_all_command_parsing() {
    use overitall::command::parse_command;

    let cmd = parse_command("show all");
    assert!(matches!(cmd, overitall::command::Command::ShowAll));
}


#[test]
fn test_show_process_restores_logs() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // First hide worker
    app.filters.hidden_processes.insert("worker".to_string());

    // Then show it again
    app.filters.hidden_processes.remove("worker");

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // Worker logs should appear again
    assert!(output.contains("Processing job") || output.contains("worker"));
}


#[test]
fn test_snapshot_hidden_process_display() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Hide worker process
    app.filters.hidden_processes.insert("worker".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_all_processes_hidden() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Hide all processes
    app.filters.hidden_processes.insert("web".to_string());
    app.filters.hidden_processes.insert("worker".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}


#[test]
fn test_snapshot_hidden_with_filters_combined() {
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Hide worker and apply filter
    app.filters.hidden_processes.insert("worker".to_string());
    app.filters.add_include_filter("ERROR".to_string());

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}
