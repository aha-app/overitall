use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::Path;

use crate::config::{self, Config};
use crate::procfile::Procfile;
use crate::skill;

/// Overitall - Process and log management TUI
#[derive(Parser, Debug)]
#[command(name = "oit")]
#[command(version)]
#[command(about = "Process and log management TUI")]
#[command(long_about = "Overitall (oit) combines process management with log viewing.

It reads a Procfile to start and manage processes, tracks their output and optional log files,
and provides an interactive TUI for viewing interleaved logs with filtering, search, and batch navigation.

Quick start:
  1. Create a Procfile with your processes (e.g., 'web: rails server')
  2. Run 'oit --init' to generate a config file
  3. Edit .overitall.toml to configure log files (optional)
  4. Run 'oit' to start the TUI

For more information, see: https://github.com/jemmyw/overitall")]
pub struct Cli {
    /// Path to config file (defaults to .overitall.toml)
    #[arg(short, long, default_value = ".overitall.toml")]
    pub config: String,

    /// Initialize a new .overitall.toml config file from Procfile
    #[arg(long)]
    pub init: bool,

    /// When using --init, also install Claude Code/Cursor skill without prompting
    #[arg(long)]
    pub with_skill: bool,

    /// Skip auto-update check on startup
    #[arg(long)]
    pub no_update: bool,

    /// Subcommand for IPC client operations
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// CLI subcommands for communicating with a running TUI instance
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Check if TUI is running (returns pong on success)
    Ping,
    /// Get status from running TUI (version, running state)
    Status,
    /// List all processes and their current status
    Processes,
    /// Get recent log lines from the TUI
    Logs {
        /// Maximum number of log lines to return (default: 100)
        #[arg(long, default_value = "100")]
        limit: u64,
        /// Number of log lines to skip (default: 0)
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Search log lines for a pattern
    Search {
        /// The search pattern (substring match)
        pattern: String,
        /// Maximum number of matches to return (default: 100)
        #[arg(long, default_value = "100")]
        limit: u64,
        /// Enable case-sensitive matching (default: case-insensitive)
        #[arg(long)]
        case_sensitive: bool,
    },
    /// Select a log line by ID and open expanded view in TUI
    Select {
        /// The log line ID to select (from search results)
        id: u64,
    },
    /// Get context lines around a specific log line ID
    Context {
        /// The log line ID to get context for
        id: u64,
        /// Number of lines before the target (default: 5)
        #[arg(long, default_value = "5")]
        before: u64,
        /// Number of lines after the target (default: 5)
        #[arg(long, default_value = "5")]
        after: u64,
    },
    /// List available IPC commands (alias for 'oit commands')
    #[command(name = "commands")]
    IpcHelp,
    /// Get trace recording status and active trace info
    Trace,
    /// Jump to a specific log line by ID (scrolls view without expanding)
    Goto {
        /// The log line ID to scroll to (from search or logs output)
        id: u64,
    },
    /// Scroll the log view up, down, to top, or to bottom
    Scroll {
        /// Scroll direction: up, down, top, or bottom
        direction: String,
        /// Number of lines to scroll (for up/down, default: 20)
        #[arg(long, default_value = "20")]
        lines: u64,
    },
    /// Freeze or unfreeze the TUI display (pauses auto-scroll)
    Freeze {
        /// Mode: on, off, or toggle (default: toggle)
        #[arg(default_value = "toggle")]
        mode: String,
    },
    /// List current filters
    Filters,
    /// Add a new filter (persists to config file)
    FilterAdd {
        /// The filter pattern to match
        pattern: String,
        /// Exclude matching lines instead of including them
        #[arg(long)]
        exclude: bool,
    },
    /// Remove a filter by pattern (persists to config file)
    FilterRemove {
        /// The filter pattern to remove
        pattern: String,
    },
    /// Clear all filters (persists to config file)
    FilterClear,
    /// List visibility status for all processes (which are shown/hidden)
    Visibility,
    /// Hide a process from log view (runtime only, does not persist)
    Hide {
        /// Process name to hide
        name: String,
    },
    /// Show a hidden process (runtime only, does not persist)
    Show {
        /// Process name to show
        name: String,
    },
    /// Restart a process or all processes
    #[command(visible_alias = "r")]
    Restart {
        /// Process name to restart (restarts all if omitted)
        name: Option<String>,
    },
    /// Kill a running process
    #[command(visible_alias = "k")]
    Kill {
        /// Process name to kill
        name: String,
    },
    /// Start a stopped process
    #[command(visible_alias = "s")]
    Start {
        /// Process name to start
        name: String,
    },
    /// Get recent log lines containing error or warning patterns
    Errors {
        /// Maximum number of lines to return (default: 50)
        #[arg(long, default_value = "50")]
        limit: u64,
        /// Level filter: error, warning, or error_or_warning (default: error)
        #[arg(long, default_value = "error")]
        level: String,
        /// Filter by process name
        #[arg(long)]
        process: Option<String>,
    },
    /// Get comprehensive AI-friendly summary of current state
    Summary,
    /// Get all log lines from a specific batch
    Batch {
        /// Batch ID to retrieve
        id: u64,
        /// Scroll TUI to first line of batch
        #[arg(long)]
        scroll: bool,
    },
}

/// Initialize a new config file from an existing Procfile
pub fn init_config(config_path: &str, with_skill: bool) -> anyhow::Result<()> {
    let config_exists = Path::new(config_path).exists();

    // Only create config if it doesn't exist
    if !config_exists {
        // Default Procfile location
        let procfile_path = "Procfile";

        // Check if Procfile exists and provide helpful error if not
        if !Path::new(procfile_path).exists() {
            return Err(anyhow!(
                "No Procfile found in current directory.\n\n\
                To use --init, first create a Procfile with your processes.\n\
                Example Procfile:\n\
                \n\
                  web: rails server -p 3000\n\
                  worker: bundle exec sidekiq\n\
                \n\
                See: https://devcenter.heroku.com/articles/procfile\n\
                \n\
                Then run 'oit --init' again to generate the config file."
            ));
        }

        // Try to parse the Procfile
        let procfile = Procfile::from_file(procfile_path)
            .with_context(|| format!("Failed to parse Procfile at '{}'", procfile_path))?;

        // Get sorted list of process names
        let process_names = procfile.process_names();

        // Create default config
        let mut processes = HashMap::new();
        for name in &process_names {
            processes.insert(
                name.to_string(),
                config::ProcessConfig {
                    log_file: Some(std::path::PathBuf::from(format!("logs/{}.log", name))),
                },
            );
        }

        let config = Config {
            procfile: std::path::PathBuf::from(procfile_path),
            processes,
            filters: config::FilterConfig {
                include: vec![],
                exclude: vec![],
            },
            batch_window_ms: Some(100),
            max_log_buffer_mb: Some(50),
            hidden_processes: Vec::new(),
            disable_auto_update: None,
            compact_mode: None,
            config_path: None,
        };

        // Save the config
        config.save(config_path)
            .with_context(|| format!("Failed to write config to '{}'", config_path))?;

        // Print success message
        println!("Created {} with {} processes:", config_path, process_names.len());
        for name in &process_names {
            println!("  - {}", name);
        }
    } else {
        println!("Config file '{}' already exists, skipping config creation.", config_path);
    }

    // Handle skill installation
    let mut skill_installed = false;
    if let Some(ai_dir) = skill::detect_ai_tool_directory() {
        let should_install = if with_skill {
            true
        } else {
            skill::prompt_skill_install(ai_dir).unwrap_or(false)
        };

        if should_install {
            match skill::install_skill(ai_dir) {
                Ok(()) => {
                    println!("\nInstalled {}/skills/oit/", ai_dir);
                    skill_installed = true;
                }
                Err(e) => {
                    eprintln!("\nWarning: Failed to install skill: {}", e);
                }
            }
        }
    }

    if !config_exists || skill_installed {
        println!("\nNext steps:");
        let mut step = 1;
        if !config_exists {
            println!("  {}. Edit {} to configure log file paths", step, config_path);
            step += 1;
            println!("  {}. Run 'oit' to start the TUI", step);
            step += 1;
        }
        if skill_installed {
            println!("  {}. AI tools can now control oit via CLI commands", step);
        }
    }

    Ok(())
}

/// Get the default IPC socket path
pub fn get_socket_path() -> std::path::PathBuf {
    // Use current directory to allow multiple instances in different directories
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".oit.sock")
}

/// Check if another oit instance is already running.
/// Returns Ok(true) if running, Ok(false) if not running (stale socket removed),
/// or Err if there was an unexpected error.
pub async fn check_already_running() -> anyhow::Result<bool> {
    use crate::ipc::{IpcClient, IpcRequest};
    use std::time::Duration;

    let socket_path = get_socket_path();

    // If socket file doesn't exist, nothing is running
    if !socket_path.exists() {
        return Ok(false);
    }

    // Try to connect and send a ping with a short timeout
    let connect_result = tokio::time::timeout(
        Duration::from_millis(500),
        IpcClient::connect(&socket_path),
    )
    .await;

    match connect_result {
        Ok(Ok(mut client)) => {
            // Connected, try to ping
            let request = IpcRequest::new("ping");
            let ping_result = tokio::time::timeout(
                Duration::from_millis(500),
                client.call(&request),
            )
            .await;

            match ping_result {
                Ok(Ok(response)) if response.success => {
                    // Server responded to ping - it's running
                    Ok(true)
                }
                _ => {
                    // Connected but no valid response - stale socket
                    let _ = std::fs::remove_file(&socket_path);
                    Ok(false)
                }
            }
        }
        Ok(Err(_)) | Err(_) => {
            // Connection failed or timed out - stale socket
            let _ = std::fs::remove_file(&socket_path);
            Ok(false)
        }
    }
}

/// Run an IPC command and print the result
pub async fn run_ipc_command(command: &Commands) -> anyhow::Result<()> {
    use crate::ipc::{IpcClient, IpcRequest};

    let socket_path = get_socket_path();

    let mut client = IpcClient::connect(&socket_path)
        .await
        .with_context(|| {
            format!(
                "Could not connect to TUI at {:?}. Is 'oit' running?",
                socket_path
            )
        })?;

    let request = match command {
        Commands::Ping => IpcRequest::new("ping"),
        Commands::Status => IpcRequest::new("status"),
        Commands::Processes => IpcRequest::new("processes"),
        Commands::Logs { limit, offset } => {
            IpcRequest::with_args("logs", serde_json::json!({"limit": limit, "offset": offset}))
        }
        Commands::Search {
            pattern,
            limit,
            case_sensitive,
        } => IpcRequest::with_args(
            "search",
            serde_json::json!({
                "pattern": pattern,
                "limit": limit,
                "case_sensitive": case_sensitive
            }),
        ),
        Commands::Select { id } => {
            IpcRequest::with_args("select", serde_json::json!({"id": id}))
        }
        Commands::Context { id, before, after } => IpcRequest::with_args(
            "context",
            serde_json::json!({"id": id, "before": before, "after": after}),
        ),
        Commands::IpcHelp => IpcRequest::new("help"),
        Commands::Trace => IpcRequest::new("trace"),
        Commands::Goto { id } => IpcRequest::with_args("goto", serde_json::json!({"id": id})),
        Commands::Scroll { direction, lines } => IpcRequest::with_args(
            "scroll",
            serde_json::json!({"direction": direction, "lines": lines}),
        ),
        Commands::Freeze { mode } => {
            IpcRequest::with_args("freeze", serde_json::json!({"mode": mode}))
        }
        Commands::Filters => IpcRequest::new("filters"),
        Commands::FilterAdd { pattern, exclude } => IpcRequest::with_args(
            "filter_add",
            serde_json::json!({"pattern": pattern, "exclude": exclude}),
        ),
        Commands::FilterRemove { pattern } => {
            IpcRequest::with_args("filter_remove", serde_json::json!({"pattern": pattern}))
        }
        Commands::FilterClear => IpcRequest::new("filter_clear"),
        Commands::Visibility => IpcRequest::new("visibility"),
        Commands::Hide { name } => {
            IpcRequest::with_args("hide", serde_json::json!({"name": name}))
        }
        Commands::Show { name } => {
            IpcRequest::with_args("show", serde_json::json!({"name": name}))
        }
        Commands::Restart { name } => {
            let args = match name {
                Some(n) => serde_json::json!({"name": n}),
                None => serde_json::json!({}),
            };
            IpcRequest::with_args("restart", args)
        }
        Commands::Kill { name } => {
            IpcRequest::with_args("kill", serde_json::json!({"name": name}))
        }
        Commands::Start { name } => {
            IpcRequest::with_args("start", serde_json::json!({"name": name}))
        }
        Commands::Errors {
            limit,
            level,
            process,
        } => {
            let mut args = serde_json::json!({"limit": limit, "level": level});
            if let Some(proc) = process {
                args["process"] = serde_json::json!(proc);
            }
            IpcRequest::with_args("errors", args)
        }
        Commands::Summary => IpcRequest::new("summary"),
        Commands::Batch { id, scroll } => {
            IpcRequest::with_args("batch", serde_json::json!({"id": id, "scroll": scroll}))
        }
    };

    let response = client.call(&request).await.with_context(|| {
        format!("Failed to communicate with TUI at {:?}", socket_path)
    })?;

    // Print response as JSON
    let json = serde_json::to_string_pretty(&response)
        .with_context(|| "Failed to serialize response")?;
    println!("{}", json);

    // Exit with error code if command failed
    if !response.success {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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

        // Call init_config (with_skill=false since no .claude dir exists)
        let result = init_config(config_path.to_str().unwrap(), false);

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
        let result = init_config(config_path.to_str().unwrap(), false);

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
        let result = init_config(config_path.to_str().unwrap(), false);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Check that init failed
        assert!(result.is_err(), "init_config should fail when Procfile is missing");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Procfile"), "Error should mention Procfile: {}", err_msg);
    }

    #[test]
    fn test_init_config_with_skill_installs_skill_files() {
        let _guard = CWD_MUTEX.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create Procfile
        fs::write(temp_path.join("Procfile"), "web: rails server\n").unwrap();

        // Create .claude directory (simulate Claude Code environment)
        fs::create_dir(temp_path.join(".claude")).unwrap();

        let config_path = temp_path.join(".overitall.toml");

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = init_config(config_path.to_str().unwrap(), true);

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok(), "init_config with --with-skill should succeed: {:?}", result.err());

        // Verify skill files were created
        let skill_md = temp_path.join(".claude/skills/oit/SKILL.md");
        let commands_md = temp_path.join(".claude/skills/oit/COMMANDS.md");

        assert!(skill_md.exists(), "SKILL.md should be created");
        assert!(commands_md.exists(), "COMMANDS.md should be created");

        let skill_content = fs::read_to_string(&skill_md).unwrap();
        assert!(skill_content.contains("name: oit"), "SKILL.md should contain skill name");
    }

    #[test]
    fn test_init_config_without_skill_flag_no_skill_installed() {
        let _guard = CWD_MUTEX.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("Procfile"), "web: rails server\n").unwrap();
        fs::create_dir(temp_path.join(".claude")).unwrap();

        let config_path = temp_path.join(".overitall.toml");

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // with_skill=false and non-TTY means no prompt, no install
        let result = init_config(config_path.to_str().unwrap(), false);

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());

        // Skill files should not be created (no TTY prompt, with_skill=false)
        let skill_dir = temp_path.join(".claude/skills/oit");
        assert!(!skill_dir.exists(), "Skill directory should not be created without --with-skill");
    }

    #[test]
    fn test_init_config_no_claude_dir_no_skill_installed() {
        let _guard = CWD_MUTEX.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("Procfile"), "web: rails server\n").unwrap();
        // No .claude directory

        let config_path = temp_path.join(".overitall.toml");

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = init_config(config_path.to_str().unwrap(), true);

        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());

        // Skill files should not be created (no .claude directory)
        let skill_dir = temp_path.join(".claude/skills/oit");
        assert!(!skill_dir.exists(), "Skill should not be installed without .claude directory");
    }

    #[test]
    fn test_cli_parses_with_skill_flag() {
        let cli = Cli::parse_from(["oit", "--init", "--with-skill"]);
        assert!(cli.init);
        assert!(cli.with_skill);
    }

    #[test]
    fn test_cli_with_skill_defaults_to_false() {
        let cli = Cli::parse_from(["oit", "--init"]);
        assert!(cli.init);
        assert!(!cli.with_skill);
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
}
