use chrono::{Local, TimeZone};
use overitall::{
    log::{LogLine, LogSource},
    process::ProcessManager,
    ui::App,
};
use ratatui::{backend::TestBackend, Terminal};

/// Helper to create an App with test data for testing
pub fn create_test_app() -> App {
    App::new()
}

/// Helper to create a ProcessManager with test data
pub fn create_test_process_manager() -> ProcessManager {
    ProcessManager::new()
}

/// Helper to create a test log line with fixed timestamp
pub fn create_test_log_line(process: &str, message: &str) -> LogLine {
    let fixed_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();
    LogLine::new_with_time(LogSource::ProcessStdout(process.to_string()), message.to_string(), fixed_time)
}

/// Helper to create a ProcessManager with test logs
pub fn create_manager_with_logs() -> ProcessManager {
    let mut manager = ProcessManager::new();

    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);
    manager.add_process("worker".to_string(), "ruby worker.rb".to_string(), None, None);

    manager.add_test_log(create_test_log_line("web", "Starting web server on port 3000"));
    manager.add_test_log(create_test_log_line("web", "GET /api/users 200 OK"));
    manager.add_test_log(create_test_log_line("worker", "Processing job #1234"));
    manager.add_test_log(create_test_log_line("web", "ERROR: Database connection failed"));
    manager.add_test_log(create_test_log_line("worker", "Job #1234 completed successfully"));
    manager.add_test_log(create_test_log_line("web", "POST /api/auth 201 Created"));
    manager.add_test_log(create_test_log_line("worker", "ERROR: Failed to process job #5678"));
    manager.add_test_log(create_test_log_line("web", "Server ready to accept connections"));

    manager
}

/// Helper to render the app to a test terminal and return the buffer as a string
pub fn render_app_to_string(app: &mut App, manager: &ProcessManager, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            overitall::ui::draw(f, app, manager);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut result = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = buffer.cell((x, y)).unwrap();
            result.push_str(cell.symbol());
        }
        result.push('\n');
    }
    result
}

/// Helper to create logs with specific arrival times for batch testing
pub fn create_manager_with_batched_logs() -> ProcessManager {
    let mut manager = ProcessManager::new();

    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);
    manager.add_process("worker".to_string(), "ruby worker.rb".to_string(), None, None);

    // Batch 1: Three logs arriving within 100ms (at 12:00:00.000)
    let batch1_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();
    let log1 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "Starting web server on port 3000".to_string(), batch1_time);
    manager.add_test_log(log1);

    let mut log2 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "Loading configuration".to_string(), batch1_time);
    log2.arrival_time = batch1_time + chrono::Duration::milliseconds(50);
    manager.add_test_log(log2);

    let mut log3 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "Database connected".to_string(), batch1_time);
    log3.arrival_time = batch1_time + chrono::Duration::milliseconds(90);
    manager.add_test_log(log3);

    // Batch 2: Two logs arriving 500ms later (at 12:00:00.500)
    let batch2_time = batch1_time + chrono::Duration::milliseconds(500);
    let log4 = LogLine::new_with_time(LogSource::ProcessStdout("worker".to_string()), "Processing job #1234".to_string(), batch2_time);
    manager.add_test_log(log4);

    let mut log5 = LogLine::new_with_time(LogSource::ProcessStdout("worker".to_string()), "Job #1234 completed".to_string(), batch2_time);
    log5.arrival_time = batch2_time + chrono::Duration::milliseconds(80);
    manager.add_test_log(log5);

    // Batch 3: Single log 1 second later (at 12:00:01.500)
    let batch3_time = batch2_time + chrono::Duration::milliseconds(1000);
    let log6 = LogLine::new_with_time(LogSource::ProcessStdout("web".to_string()), "GET /api/users 200 OK".to_string(), batch3_time);
    manager.add_test_log(log6);

    manager
}

/// Helper to create a manager with exactly N logs that arrive together (same batch)
pub fn create_manager_with_n_logs_same_batch(n: usize) -> ProcessManager {
    let mut manager = ProcessManager::new();
    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);

    let base_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

    for i in 0..n {
        let mut log = LogLine::new_with_time(
            LogSource::ProcessStdout("web".to_string()),
            format!("Log line number {}", i + 1),
            base_time,
        );
        log.arrival_time = base_time + chrono::Duration::milliseconds(i as i64);
        manager.add_test_log(log);
    }

    manager
}

/// Helper to create a manager with N logs that arrive in separate batches
pub fn create_manager_with_n_logs_separate_batches(n: usize) -> ProcessManager {
    let mut manager = ProcessManager::new();
    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);

    let base_time = Local.with_ymd_and_hms(2024, 12, 10, 12, 0, 0).unwrap();

    for i in 0..n {
        let mut log = LogLine::new_with_time(
            LogSource::ProcessStdout("web".to_string()),
            format!("Log line number {}", i + 1),
            base_time,
        );
        log.arrival_time = base_time + chrono::Duration::seconds(i as i64);
        manager.add_test_log(log);
    }

    manager
}

/// Helper to create a ProcessManager with custom status configuration
pub fn create_manager_with_custom_status() -> ProcessManager {
    use overitall::config::{StatusConfig, StatusTransition};

    let status_config = StatusConfig {
        default: Some("Starting".to_string()),
        color: None,
        transitions: vec![
            StatusTransition {
                pattern: "Ready".to_string(),
                label: "Ready".to_string(),
                color: Some("green".to_string()),
            },
        ],
    };

    let mut manager = ProcessManager::new();
    manager.add_process("web".to_string(), "echo hi".to_string(), None, Some(&status_config));
    manager.add_process("worker".to_string(), "echo hi".to_string(), None, None);

    // Simulate process start to apply default custom status
    manager.reset_process_status("web");

    manager
}

/// Helper to create a ProcessManager with long log lines for testing wrap/truncate
pub fn create_manager_with_long_logs() -> ProcessManager {
    let mut manager = ProcessManager::new();

    manager.add_process("web".to_string(), "ruby web.rb".to_string(), None, None);
    manager.add_process("worker".to_string(), "ruby worker.rb".to_string(), None, None);

    manager.add_test_log(create_test_log_line("web", "Short log message"));
    manager.add_test_log(create_test_log_line("web", "This is a much longer log message that will definitely exceed the terminal width and need to be either truncated or wrapped depending on the display mode setting"));
    manager.add_test_log(create_test_log_line("worker", "Processing job #1234"));
    manager.add_test_log(create_test_log_line("worker", "ERROR: Failed to connect to database at host=db.example.com port=5432 user=app_user database=production reason=connection_refused after_attempts=3 retry_delay_ms=1000"));
    manager.add_test_log(create_test_log_line("web", "GET /api/users HTTP/1.1 200 OK response_time=45ms user_agent=Mozilla/5.0 referer=https://example.com/dashboard"));

    manager
}

/// Helper to create a ProcessManager with mixed process states for testing Summary mode
/// Creates processes with running, stopped, failed, and custom status states
pub fn create_manager_with_mixed_states() -> ProcessManager {
    use overitall::config::{StatusConfig, StatusTransition};
    use overitall::process::ProcessStatus;

    let mut manager = ProcessManager::new();

    // Running process (not noteworthy)
    manager.add_process("web".to_string(), "echo hi".to_string(), None, None);

    // Another running process (not noteworthy)
    manager.add_process("api".to_string(), "echo hi".to_string(), None, None);

    // Running process with custom status (noteworthy)
    let db_config = StatusConfig {
        default: Some("Syncing".to_string()),
        color: None,
        transitions: vec![
            StatusTransition {
                pattern: "Ready".to_string(),
                label: "Ready".to_string(),
                color: Some("green".to_string()),
            },
        ],
    };
    manager.add_process("db".to_string(), "echo hi".to_string(), None, Some(&db_config));
    manager.reset_process_status("db"); // Apply default custom status

    // Stopped process (noteworthy)
    manager.add_process("worker".to_string(), "echo hi".to_string(), None, None);
    manager.set_process_status_for_testing("worker", ProcessStatus::Stopped);

    // Failed process (noteworthy)
    manager.add_process("mailer".to_string(), "echo hi".to_string(), None, None);
    manager.set_process_status_for_testing("mailer", ProcessStatus::Failed("Exit code 1".to_string()));

    manager
}

/// Helper to create a ProcessManager with many processes for testing grid layout
/// Creates 12 processes with varied names and statuses to span multiple rows
pub fn create_manager_with_many_processes() -> ProcessManager {
    use overitall::config::{StatusConfig, StatusTransition};

    let mut manager = ProcessManager::new();

    // Process with custom status
    let web_config = StatusConfig {
        default: Some("Booting".to_string()),
        color: None,
        transitions: vec![
            StatusTransition {
                pattern: "Ready".to_string(),
                label: "Ready".to_string(),
                color: Some("green".to_string()),
            },
        ],
    };
    manager.add_process("web".to_string(), "echo hi".to_string(), None, Some(&web_config));
    manager.reset_process_status("web"); // Apply default custom status

    // Process with longer custom status
    let api_config = StatusConfig {
        default: Some("Initializing".to_string()),
        color: None,
        transitions: vec![],
    };
    manager.add_process("api".to_string(), "echo hi".to_string(), None, Some(&api_config));
    manager.reset_process_status("api"); // Apply default custom status

    // Regular processes (no custom status)
    manager.add_process("worker".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("scheduler".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("mailer".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("cache".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("db".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("redis".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("nginx".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("postgres".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("elasticsearch".to_string(), "echo hi".to_string(), None, None);
    manager.add_process("sidekiq".to_string(), "echo hi".to_string(), None, None);

    manager
}
