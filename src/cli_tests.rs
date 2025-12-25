use super::*;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

// Mutex to serialize tests that change the current directory
// This prevents race conditions when tests run in parallel
static CWD_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_init_config_creates_file() {
    // Lock mutex to prevent parallel directory changes
    let _guard = CWD_MUTEX.lock().unwrap();

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a test Procfile
    let procfile_path = temp_path.join("Procfile");
    fs::write(&procfile_path, "web: rails server\nworker: sidekiq\n").unwrap();

    // Create a config file path
    let config_path = temp_path.join(".overitall.toml");

    // Change to the temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_path).unwrap();

    // Call init_config
    let result = init_config(config_path.to_str().unwrap(), None);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Check that init succeeded
    assert!(result.is_ok(), "init_config should succeed: {:?}", result.err());

    // Check that the config file was created
    assert!(config_path.exists(), "Config file should be created");

    // Read and verify the config
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("procfile = \"Procfile\""));
    assert!(config_content.contains("[processes.web]"));
    assert!(config_content.contains("[processes.worker]"));
    assert!(config_content.contains("batch_window_ms = 100"));
}

#[test]
fn test_init_config_skips_config_if_file_exists() {
    // Lock mutex to prevent parallel directory changes
    let _guard = CWD_MUTEX.lock().unwrap();

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a config file that already exists with custom content
    let config_path = temp_path.join(".overitall.toml");
    let original_content = "# existing config\n";
    fs::write(&config_path, original_content).unwrap();

    // Change to the temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_path).unwrap();

    // Call init_config (no Procfile needed since config exists)
    let result = init_config(config_path.to_str().unwrap(), None);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Check that init succeeded but didn't overwrite the config
    assert!(result.is_ok(), "init_config should succeed when config exists");
    let content = fs::read_to_string(&config_path).unwrap();
    assert_eq!(content, original_content, "Config file should not be modified");
}

#[test]
fn test_init_config_fails_if_procfile_missing() {
    // Lock mutex to prevent parallel directory changes
    let _guard = CWD_MUTEX.lock().unwrap();

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a config file path (but no Procfile)
    let config_path = temp_path.join(".overitall.toml");

    // Change to the temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_path).unwrap();

    // Call init_config
    let result = init_config(config_path.to_str().unwrap(), None);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Check that init failed
    assert!(result.is_err(), "init_config should fail when Procfile is missing");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Procfile"), "Error should mention Procfile: {}", err_msg);
}

#[test]
fn test_cli_parses_default_config() {
    let cli = Cli::parse_from(["oit"]);
    assert_eq!(cli.config, ".overitall.toml");
    assert!(!cli.init);
}

#[test]
fn test_cli_parses_custom_config() {
    let cli = Cli::parse_from(["oit", "-c", "custom.toml"]);
    assert_eq!(cli.config, "custom.toml");
    assert!(!cli.init);
}

#[test]
fn test_cli_parses_long_config_flag() {
    let cli = Cli::parse_from(["oit", "--config", "path/to/config.toml"]);
    assert_eq!(cli.config, "path/to/config.toml");
}

#[test]
fn test_cli_parses_init_flag() {
    let cli = Cli::parse_from(["oit", "--init"]);
    assert!(cli.init);
}

#[test]
fn test_cli_parses_init_with_custom_config() {
    let cli = Cli::parse_from(["oit", "--init", "-c", "custom.toml"]);
    assert!(cli.init);
    assert_eq!(cli.config, "custom.toml");
}

#[test]
fn test_cli_parses_procfile_short_flag() {
    let cli = Cli::parse_from(["oit", "-f", "Procfile.dev"]);
    assert_eq!(cli.procfile, Some("Procfile.dev".to_string()));
}

#[test]
fn test_cli_parses_procfile_long_flag() {
    let cli = Cli::parse_from(["oit", "--file", "Procfile.test"]);
    assert_eq!(cli.procfile, Some("Procfile.test".to_string()));
}

#[test]
fn test_cli_default_procfile_is_none() {
    let cli = Cli::parse_from(["oit"]);
    assert!(cli.procfile.is_none());
}

#[test]
fn test_cli_parses_procfile_with_init() {
    let cli = Cli::parse_from(["oit", "--init", "-f", "Procfile.custom"]);
    assert!(cli.init);
    assert_eq!(cli.procfile, Some("Procfile.custom".to_string()));
}

#[test]
fn test_init_config_with_custom_procfile() {
    let _guard = CWD_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a custom Procfile
    let procfile_path = temp_path.join("Procfile.dev");
    fs::write(&procfile_path, "api: node server.js\n").unwrap();

    let config_path = temp_path.join(".overitall.toml");

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_path).unwrap();

    // Call init_config with custom procfile
    let result = init_config(config_path.to_str().unwrap(), Some("Procfile.dev"));

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok(), "init_config should succeed: {:?}", result.err());
    assert!(config_path.exists(), "Config file should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("procfile = \"Procfile.dev\""));
    assert!(config_content.contains("[processes.api]"));
}

#[test]
fn test_cli_parses_no_update_flag() {
    let cli = Cli::parse_from(["oit", "--no-update"]);
    assert!(cli.no_update);
    assert!(!cli.init);
}

#[test]
fn test_cli_default_no_update_is_false() {
    let cli = Cli::parse_from(["oit"]);
    assert!(!cli.no_update);
}

#[test]
fn test_cli_parses_update_flag() {
    let cli = Cli::parse_from(["oit", "--update"]);
    assert!(cli.update);
}

#[test]
fn test_cli_default_update_is_false() {
    let cli = Cli::parse_from(["oit"]);
    assert!(!cli.update);
}

#[test]
fn test_cli_parses_ping_subcommand() {
    let cli = Cli::parse_from(["oit", "ping"]);
    assert!(matches!(cli.command, Some(Commands::Ping)));
}

#[test]
fn test_cli_parses_status_subcommand() {
    let cli = Cli::parse_from(["oit", "status"]);
    assert!(matches!(cli.command, Some(Commands::Status)));
}

#[test]
fn test_cli_parses_processes_subcommand() {
    let cli = Cli::parse_from(["oit", "processes"]);
    assert!(matches!(cli.command, Some(Commands::Processes)));
}

#[test]
fn test_cli_parses_logs_subcommand() {
    let cli = Cli::parse_from(["oit", "logs"]);
    match cli.command {
        Some(Commands::Logs { limit, offset }) => {
            assert_eq!(limit, 100);
            assert_eq!(offset, 0);
        }
        _ => panic!("Expected Logs command"),
    }
}

#[test]
fn test_cli_parses_logs_with_limit_and_offset() {
    let cli = Cli::parse_from(["oit", "logs", "--limit", "50", "--offset", "10"]);
    match cli.command {
        Some(Commands::Logs { limit, offset }) => {
            assert_eq!(limit, 50);
            assert_eq!(offset, 10);
        }
        _ => panic!("Expected Logs command"),
    }
}

#[test]
fn test_cli_no_subcommand_by_default() {
    let cli = Cli::parse_from(["oit"]);
    assert!(cli.command.is_none());
}

#[test]
fn test_get_socket_path_uses_current_dir() {
    let path = get_socket_path();
    let expected = std::env::current_dir()
        .unwrap()
        .join(".oit.sock");
    assert_eq!(path, expected);
}

#[test]
fn test_get_socket_path_filename_is_hidden() {
    let path = get_socket_path();
    let filename = path.file_name().unwrap().to_str().unwrap();
    assert!(filename.starts_with('.'), "Socket filename should be hidden (start with .)");
    assert_eq!(filename, ".oit.sock");
}

#[test]
fn test_cli_parses_search_subcommand() {
    let cli = Cli::parse_from(["oit", "search", "error"]);
    match cli.command {
        Some(Commands::Search {
            pattern,
            limit,
            case_sensitive,
        }) => {
            assert_eq!(pattern, "error");
            assert_eq!(limit, 100);
            assert!(!case_sensitive);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_cli_parses_search_with_options() {
    let cli = Cli::parse_from(["oit", "search", "ERROR", "--limit", "50", "--case-sensitive"]);
    match cli.command {
        Some(Commands::Search {
            pattern,
            limit,
            case_sensitive,
        }) => {
            assert_eq!(pattern, "ERROR");
            assert_eq!(limit, 50);
            assert!(case_sensitive);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_cli_parses_commands_subcommand() {
    let cli = Cli::parse_from(["oit", "commands"]);
    assert!(matches!(cli.command, Some(Commands::IpcHelp)));
}

#[test]
fn test_cli_parses_trace_subcommand() {
    let cli = Cli::parse_from(["oit", "trace"]);
    assert!(matches!(cli.command, Some(Commands::Trace)));
}

#[test]
fn test_cli_parses_goto_subcommand() {
    let cli = Cli::parse_from(["oit", "goto", "42"]);
    match cli.command {
        Some(Commands::Goto { id }) => {
            assert_eq!(id, 42);
        }
        _ => panic!("Expected Goto command"),
    }
}

#[test]
fn test_cli_parses_scroll_subcommand() {
    let cli = Cli::parse_from(["oit", "scroll", "up"]);
    match cli.command {
        Some(Commands::Scroll { direction, lines }) => {
            assert_eq!(direction, "up");
            assert_eq!(lines, 20); // default
        }
        _ => panic!("Expected Scroll command"),
    }
}

#[test]
fn test_cli_parses_scroll_with_lines() {
    let cli = Cli::parse_from(["oit", "scroll", "down", "--lines", "50"]);
    match cli.command {
        Some(Commands::Scroll { direction, lines }) => {
            assert_eq!(direction, "down");
            assert_eq!(lines, 50);
        }
        _ => panic!("Expected Scroll command"),
    }
}

#[test]
fn test_cli_parses_scroll_top() {
    let cli = Cli::parse_from(["oit", "scroll", "top"]);
    match cli.command {
        Some(Commands::Scroll { direction, lines: _ }) => {
            assert_eq!(direction, "top");
        }
        _ => panic!("Expected Scroll command"),
    }
}

#[test]
fn test_cli_parses_scroll_bottom() {
    let cli = Cli::parse_from(["oit", "scroll", "bottom"]);
    match cli.command {
        Some(Commands::Scroll { direction, lines: _ }) => {
            assert_eq!(direction, "bottom");
        }
        _ => panic!("Expected Scroll command"),
    }
}

#[test]
fn test_cli_parses_freeze_subcommand() {
    let cli = Cli::parse_from(["oit", "freeze"]);
    match cli.command {
        Some(Commands::Freeze { mode }) => {
            assert_eq!(mode, "toggle"); // default
        }
        _ => panic!("Expected Freeze command"),
    }
}

#[test]
fn test_cli_parses_freeze_on() {
    let cli = Cli::parse_from(["oit", "freeze", "on"]);
    match cli.command {
        Some(Commands::Freeze { mode }) => {
            assert_eq!(mode, "on");
        }
        _ => panic!("Expected Freeze command"),
    }
}

#[test]
fn test_cli_parses_freeze_off() {
    let cli = Cli::parse_from(["oit", "freeze", "off"]);
    match cli.command {
        Some(Commands::Freeze { mode }) => {
            assert_eq!(mode, "off");
        }
        _ => panic!("Expected Freeze command"),
    }
}

#[test]
fn test_cli_parses_freeze_toggle() {
    let cli = Cli::parse_from(["oit", "freeze", "toggle"]);
    match cli.command {
        Some(Commands::Freeze { mode }) => {
            assert_eq!(mode, "toggle");
        }
        _ => panic!("Expected Freeze command"),
    }
}

#[test]
fn test_cli_parses_filters_subcommand() {
    let cli = Cli::parse_from(["oit", "filters"]);
    assert!(matches!(cli.command, Some(Commands::Filters)));
}

#[test]
fn test_cli_parses_filter_add_subcommand() {
    let cli = Cli::parse_from(["oit", "filter-add", "error"]);
    match cli.command {
        Some(Commands::FilterAdd { pattern, exclude }) => {
            assert_eq!(pattern, "error");
            assert!(!exclude);
        }
        _ => panic!("Expected FilterAdd command"),
    }
}

#[test]
fn test_cli_parses_filter_add_with_exclude() {
    let cli = Cli::parse_from(["oit", "filter-add", "debug", "--exclude"]);
    match cli.command {
        Some(Commands::FilterAdd { pattern, exclude }) => {
            assert_eq!(pattern, "debug");
            assert!(exclude);
        }
        _ => panic!("Expected FilterAdd command"),
    }
}

#[test]
fn test_cli_parses_filter_remove_subcommand() {
    let cli = Cli::parse_from(["oit", "filter-remove", "error"]);
    match cli.command {
        Some(Commands::FilterRemove { pattern }) => {
            assert_eq!(pattern, "error");
        }
        _ => panic!("Expected FilterRemove command"),
    }
}

#[test]
fn test_cli_parses_filter_clear_subcommand() {
    let cli = Cli::parse_from(["oit", "filter-clear"]);
    assert!(matches!(cli.command, Some(Commands::FilterClear)));
}

#[test]
fn test_cli_parses_visibility_subcommand() {
    let cli = Cli::parse_from(["oit", "visibility"]);
    assert!(matches!(cli.command, Some(Commands::Visibility)));
}

#[test]
fn test_cli_parses_hide_subcommand() {
    let cli = Cli::parse_from(["oit", "hide", "web"]);
    match cli.command {
        Some(Commands::Hide { name }) => {
            assert_eq!(name, "web");
        }
        _ => panic!("Expected Hide command"),
    }
}

#[test]
fn test_cli_parses_show_subcommand() {
    let cli = Cli::parse_from(["oit", "show", "worker"]);
    match cli.command {
        Some(Commands::Show { name }) => {
            assert_eq!(name, "worker");
        }
        _ => panic!("Expected Show command"),
    }
}

#[test]
fn test_cli_parses_restart_subcommand() {
    let cli = Cli::parse_from(["oit", "restart"]);
    match cli.command {
        Some(Commands::Restart { name }) => {
            assert!(name.is_none());
        }
        _ => panic!("Expected Restart command"),
    }
}

#[test]
fn test_cli_parses_restart_subcommand_with_name() {
    let cli = Cli::parse_from(["oit", "restart", "web"]);
    match cli.command {
        Some(Commands::Restart { name }) => {
            assert_eq!(name, Some("web".to_string()));
        }
        _ => panic!("Expected Restart command"),
    }
}

#[test]
fn test_cli_parses_kill_subcommand() {
    let cli = Cli::parse_from(["oit", "kill", "web"]);
    match cli.command {
        Some(Commands::Kill { name }) => {
            assert_eq!(name, "web");
        }
        _ => panic!("Expected Kill command"),
    }
}

#[test]
fn test_cli_parses_start_subcommand() {
    let cli = Cli::parse_from(["oit", "start", "worker"]);
    match cli.command {
        Some(Commands::Start { name }) => {
            assert_eq!(name, "worker");
        }
        _ => panic!("Expected Start command"),
    }
}

#[test]
fn test_cli_parses_restart_alias_r() {
    let cli = Cli::parse_from(["oit", "r", "web"]);
    match cli.command {
        Some(Commands::Restart { name }) => {
            assert_eq!(name, Some("web".to_string()));
        }
        _ => panic!("Expected Restart command via 'r' alias"),
    }
}

#[test]
fn test_cli_parses_kill_alias_k() {
    let cli = Cli::parse_from(["oit", "k", "web"]);
    match cli.command {
        Some(Commands::Kill { name }) => {
            assert_eq!(name, "web");
        }
        _ => panic!("Expected Kill command via 'k' alias"),
    }
}

#[test]
fn test_cli_parses_start_alias_s() {
    let cli = Cli::parse_from(["oit", "s", "worker"]);
    match cli.command {
        Some(Commands::Start { name }) => {
            assert_eq!(name, "worker");
        }
        _ => panic!("Expected Start command via 's' alias"),
    }
}

#[test]
fn test_cli_parses_errors_subcommand() {
    let cli = Cli::parse_from(["oit", "errors"]);
    match cli.command {
        Some(Commands::Errors {
            limit,
            level,
            process,
        }) => {
            assert_eq!(limit, 50);
            assert_eq!(level, "error");
            assert!(process.is_none());
        }
        _ => panic!("Expected Errors command"),
    }
}

#[test]
fn test_cli_parses_errors_with_options() {
    let cli = Cli::parse_from([
        "oit",
        "errors",
        "--limit",
        "20",
        "--level",
        "warning",
        "--process",
        "web",
    ]);
    match cli.command {
        Some(Commands::Errors {
            limit,
            level,
            process,
        }) => {
            assert_eq!(limit, 20);
            assert_eq!(level, "warning");
            assert_eq!(process, Some("web".to_string()));
        }
        _ => panic!("Expected Errors command"),
    }
}

#[test]
fn test_cli_parses_errors_error_or_warning() {
    let cli = Cli::parse_from(["oit", "errors", "--level", "error_or_warning"]);
    match cli.command {
        Some(Commands::Errors { level, .. }) => {
            assert_eq!(level, "error_or_warning");
        }
        _ => panic!("Expected Errors command"),
    }
}

#[test]
fn test_cli_parses_summary_subcommand() {
    let cli = Cli::parse_from(["oit", "summary"]);
    assert!(matches!(cli.command, Some(Commands::Summary)));
}

#[test]
fn test_cli_parses_batch_subcommand() {
    let cli = Cli::parse_from(["oit", "batch", "42"]);
    match cli.command {
        Some(Commands::Batch { id, scroll }) => {
            assert_eq!(id, 42);
            assert!(!scroll);
        }
        _ => panic!("Expected Batch command"),
    }
}

#[test]
fn test_cli_parses_batch_with_scroll() {
    let cli = Cli::parse_from(["oit", "batch", "123", "--scroll"]);
    match cli.command {
        Some(Commands::Batch { id, scroll }) => {
            assert_eq!(id, 123);
            assert!(scroll);
        }
        _ => panic!("Expected Batch command"),
    }
}

#[test]
fn test_cli_parses_vscode_install_subcommand() {
    let cli = Cli::parse_from(["oit", "vscode", "install"]);
    match cli.command {
        Some(Commands::Vscode { action: VscodeAction::Install }) => {}
        _ => panic!("Expected Vscode Install command"),
    }
}
