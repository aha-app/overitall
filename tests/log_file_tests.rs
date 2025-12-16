// End-to-end tests for log file path resolution
//
// These tests verify that relative log_file paths in config are correctly
// resolved relative to the procfile directory.
//
// Note: The FileReader currently has a limitation where it doesn't truly "tail" -
// it reads to EOF and exits. These tests focus on path resolution correctness.

use overitall::{
    config::Config,
    procfile::Procfile,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function that mimics the path resolution logic in main.rs
/// This is the logic we're testing - that log_file paths are resolved
/// relative to the procfile directory, not the current working directory.
fn resolve_log_file_path(config: &Config, config_path: &std::path::Path) -> Option<PathBuf> {
    // Calculate procfile_dir (mimics main.rs lines 73-77)
    let config_dir = config_path.parent()?;
    let procfile_abs = config_dir.join(&config.procfile);

    let procfile_dir = procfile_abs
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| config_dir.to_path_buf());

    // Get the log file path for "web" process
    let proc_config = config.processes.get("web")?;
    let log_file = proc_config.log_file.as_ref()?;

    // THIS IS THE FIX: join with procfile_dir
    Some(procfile_dir.join(log_file))
}

/// Test that log files with relative paths in config are correctly resolved
/// relative to the procfile directory, not the current working directory.
#[test]
fn test_relative_log_file_path_resolution() {
    // Create a temp directory structure:
    // temp_dir/
    //   .overitall.toml  (config with log_file = "logs/app.log")
    //   Procfile
    //   logs/
    //     app.log
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create logs subdirectory
    let logs_dir = temp_path.join("logs");
    std::fs::create_dir(&logs_dir).unwrap();

    // Create the log file
    let log_file_path = logs_dir.join("app.log");
    std::fs::File::create(&log_file_path).unwrap();

    // Create Procfile
    let procfile_path = temp_path.join("Procfile");
    std::fs::write(&procfile_path, "web: echo hello\n").unwrap();

    // Create config with RELATIVE log_file path
    let config_path = temp_path.join(".overitall.toml");
    let config_content = r#"
procfile = "Procfile"

[processes.web]
log_file = "logs/app.log"
"#;
    std::fs::write(&config_path, config_content).unwrap();

    // Load config
    let config = Config::from_file(config_path.to_str().unwrap()).unwrap();

    // Verify the config loaded the relative path
    assert_eq!(
        config.processes.get("web").unwrap().log_file.as_ref().unwrap(),
        &PathBuf::from("logs/app.log")
    );

    // Resolve the path using the same logic as main.rs
    let resolved_path = resolve_log_file_path(&config, &config_path).unwrap();

    // The resolved path should point to the actual file in temp_dir
    assert_eq!(resolved_path, log_file_path);
    assert!(resolved_path.exists(), "Resolved path should exist: {:?}", resolved_path);
}

/// Test that the resolved path is absolute and correct even when
/// the config specifies a nested relative path
#[test]
fn test_nested_relative_log_file_path() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create nested directory structure
    let nested_logs_dir = temp_path.join("var").join("log");
    std::fs::create_dir_all(&nested_logs_dir).unwrap();
    let log_file_path = nested_logs_dir.join("development.log");
    std::fs::File::create(&log_file_path).unwrap();

    // Create Procfile
    std::fs::write(temp_path.join("Procfile"), "web: rails s\n").unwrap();

    // Create config with nested relative path
    let config_path = temp_path.join(".overitall.toml");
    std::fs::write(&config_path, r#"
procfile = "Procfile"

[processes.web]
log_file = "var/log/development.log"
"#).unwrap();

    let config = Config::from_file(config_path.to_str().unwrap()).unwrap();
    let resolved_path = resolve_log_file_path(&config, &config_path).unwrap();

    assert_eq!(resolved_path, log_file_path);
    assert!(resolved_path.exists());
}

/// Test that WITHOUT joining with procfile_dir, the path would be wrong
#[test]
fn test_unresolved_path_is_different_from_resolved() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create the actual log file
    let logs_dir = temp_path.join("logs");
    std::fs::create_dir(&logs_dir).unwrap();
    let actual_log_file = logs_dir.join("app.log");
    std::fs::File::create(&actual_log_file).unwrap();

    std::fs::write(temp_path.join("Procfile"), "web: echo hello\n").unwrap();
    let config_path = temp_path.join(".overitall.toml");
    std::fs::write(&config_path, r#"
procfile = "Procfile"
[processes.web]
log_file = "logs/app.log"
"#).unwrap();

    let config = Config::from_file(config_path.to_str().unwrap()).unwrap();

    // The raw path from config (BUG: what we used to use)
    let unresolved_path = config.processes.get("web").unwrap().log_file.as_ref().unwrap();

    // The correctly resolved path (FIX: what we now use)
    let resolved_path = resolve_log_file_path(&config, &config_path).unwrap();

    // These should be different!
    assert_ne!(
        unresolved_path, &resolved_path,
        "Unresolved path {:?} should differ from resolved path {:?}",
        unresolved_path, resolved_path
    );

    // Only the resolved path actually exists
    assert!(!unresolved_path.exists(), "Unresolved relative path should not exist in CWD");
    assert!(resolved_path.exists(), "Resolved path should exist");
}

/// Test path resolution with Procfile in a subdirectory
#[test]
fn test_procfile_in_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create: temp_dir/app/Procfile and temp_dir/app/logs/app.log
    let app_dir = temp_path.join("app");
    std::fs::create_dir(&app_dir).unwrap();
    std::fs::write(app_dir.join("Procfile"), "web: rails s\n").unwrap();

    let logs_dir = app_dir.join("logs");
    std::fs::create_dir(&logs_dir).unwrap();
    let log_file_path = logs_dir.join("app.log");
    std::fs::File::create(&log_file_path).unwrap();

    // Config at temp_dir/.overitall.toml pointing to app/Procfile
    let config_path = temp_path.join(".overitall.toml");
    std::fs::write(&config_path, r#"
procfile = "app/Procfile"

[processes.web]
log_file = "logs/app.log"
"#).unwrap();

    let config = Config::from_file(config_path.to_str().unwrap()).unwrap();

    // Verify procfile path is relative
    assert_eq!(config.procfile, PathBuf::from("app/Procfile"));

    // Parse procfile to ensure it works
    let config_dir = config_path.parent().unwrap();
    let procfile_abs = config_dir.join(&config.procfile);
    let _procfile = Procfile::from_file(procfile_abs.to_str().unwrap()).unwrap();

    // Resolve log file path - should be relative to app/ directory (where Procfile is)
    let resolved_path = resolve_log_file_path(&config, &config_path).unwrap();

    assert_eq!(resolved_path, log_file_path);
    assert!(resolved_path.exists());
}
