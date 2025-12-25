mod common;

use common::*;
use insta::assert_snapshot;
use overitall::ui::DisplayMode;
use overitall::ui::display_state::TimestampMode;

// ============================================================================
// Display Mode Tests
// ============================================================================

#[test]
fn test_snapshot_display_mode_compact() {
    let mut app = create_test_app();
    app.display.display_mode = DisplayMode::Compact;
    let manager = create_manager_with_long_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_display_mode_full() {
    let mut app = create_test_app();
    app.display.display_mode = DisplayMode::Full;
    let manager = create_manager_with_long_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_display_mode_wrap() {
    let mut app = create_test_app();
    app.display.display_mode = DisplayMode::Wrap;
    let manager = create_manager_with_long_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

// ============================================================================
// Expanded Line View Tests
// ============================================================================

#[test]
fn test_snapshot_expanded_line_overlay_narrow() {
    // In narrow terminal (<160 cols), expanded view shows as modal overlay
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Select a line first
    overitall::operations::navigation::select_next_line(&mut app, &manager);
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    // Enable expanded view
    app.display.expanded_line_view = true;

    // Render at narrow width (120 < 160 threshold)
    let output = render_app_to_string(&mut app, &manager, 120, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_expanded_line_panel_wide() {
    // In wide terminal (>=160 cols), expanded view shows as side panel
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Select a line first
    overitall::operations::navigation::select_next_line(&mut app, &manager);
    overitall::operations::navigation::select_next_line(&mut app, &manager);

    // Enable expanded view
    app.display.expanded_line_view = true;

    // Render at wide width (180 >= 160 threshold)
    let output = render_app_to_string(&mut app, &manager, 180, 40);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_expanded_line_panel_no_selection() {
    // Wide terminal with expanded view but no line selected shows placeholder
    let mut app = create_test_app();
    let manager = create_manager_with_logs();

    // Don't select any line, just enable expanded view
    app.display.expanded_line_view = true;

    // Render at wide width (180 >= 160 threshold)
    let output = render_app_to_string(&mut app, &manager, 180, 40);
    assert_snapshot!(output);
}

// ============================================================================
// Timestamp Mode Tests
// ============================================================================

#[test]
fn test_snapshot_timestamp_mode_seconds() {
    let mut app = create_test_app();
    app.display.timestamp_mode = TimestampMode::Seconds;
    let manager = create_manager_with_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 20);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_timestamp_mode_milliseconds() {
    let mut app = create_test_app();
    app.display.timestamp_mode = TimestampMode::Milliseconds;
    let manager = create_manager_with_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 20);
    assert_snapshot!(output);
}

#[test]
fn test_snapshot_timestamp_mode_off() {
    let mut app = create_test_app();
    app.display.timestamp_mode = TimestampMode::Off;
    let manager = create_manager_with_logs();

    let output = render_app_to_string(&mut app, &manager, 120, 20);
    assert_snapshot!(output);
}
