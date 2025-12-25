mod common;

use common::*;
use insta::assert_snapshot;

// ============================================================================
// Follow Mode Scrolling Tests
// ============================================================================

#[test]
fn test_follow_mode_shows_last_lines() {
    // This test verifies that in follow mode (auto_scroll=true), the last logs
    // are visible at the bottom of the screen.
    //
    // Bug context: Previously the code subtracted 2 from visible_lines for
    // "borders" but the widget used Borders::NONE, causing the last 2 lines
    // to be cut off.
    //
    // We create MORE logs than fit on screen to ensure scrolling is needed,
    // then verify the last logs are visible.

    let mut app = create_test_app();
    // Create 50 logs in same batch - more than the ~34 line visible area
    // This ensures we need to scroll, and tests that follow mode shows the last lines
    let manager = create_manager_with_n_logs_same_batch(50);

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // The last log lines should be visible in follow mode
    assert!(
        output.contains("Log line number 50"),
        "Last log (line 50) should be visible in follow mode"
    );
    assert!(
        output.contains("Log line number 49"),
        "Second-to-last log (line 49) should be visible in follow mode"
    );
    assert!(
        output.contains("Log line number 48"),
        "Third-to-last log (line 48) should be visible in follow mode"
    );

    // Early logs should NOT be visible (scrolled off top)
    // Use word boundary to avoid matching "Log line number 10" etc.
    assert!(
        !output.contains("[12:00:00] web: Log line number 1\n"),
        "First log should be scrolled off in follow mode with 50 logs"
    );

    assert_snapshot!(output);
}

#[test]
fn test_follow_mode_with_batch_separators() {
    // This test verifies follow mode works correctly when batch separators are present.
    // Each log line becomes its own batch (due to 1-second spacing), so batch separators
    // are inserted between each log. This means fewer actual logs fit on screen.
    //
    // With a 35-line visible area and logs + separators taking 2 lines each (log + separator),
    // we can fit about 17-18 logs. The last log should still be visible.

    let mut app = create_test_app();
    // Create 25 logs in separate batches - with separators, this will overflow the screen
    let manager = create_manager_with_n_logs_separate_batches(25);

    let output = render_app_to_string(&mut app, &manager, 120, 40);

    // The last log line should be visible in follow mode
    assert!(
        output.contains("Log line number 25"),
        "Last log (line 25) should be visible in follow mode with batch separators"
    );
    assert!(
        output.contains("Log line number 24"),
        "Second-to-last log (line 24) should be visible"
    );

    // Verify batch separators are present
    assert!(
        output.contains("Batch"),
        "Batch separators should be visible"
    );

    assert_snapshot!(output);
}
