use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::Path;

use crate::config::{self, Config};
use crate::procfile::Procfile;

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
}

/// Initialize a new config file from an existing Procfile
pub fn init_config(config_path: &str) -> anyhow::Result<()> {
    // Check if config file already exists
    if Path::new(config_path).exists() {
        return Err(anyhow!(
            "Config file '{}' already exists. Remove it first if you want to reinitialize.",
            config_path
        ));
    }

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
    println!("\nNext steps:");
    println!("  1. Edit {} to configure log file paths", config_path);
    println!("  2. Run 'oit' to start the TUI");

    Ok(())
}

/// Get the default IPC socket path
pub fn get_socket_path() -> std::path::PathBuf {
    // Use current directory to allow multiple instances in different directories
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".oit.sock")
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

        // Call init_config
        let result = init_config(config_path.to_str().unwrap());

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
    fn test_init_config_fails_if_file_exists() {
        // Lock mutex to prevent parallel directory changes
        let _guard = CWD_MUTEX.lock().unwrap();

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test Procfile
        let procfile_path = temp_path.join("Procfile");
        fs::write(&procfile_path, "web: rails server\n").unwrap();

        // Create a config file that already exists
        let config_path = temp_path.join(".overitall.toml");
        fs::write(&config_path, "# existing config\n").unwrap();

        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        // Call init_config
        let result = init_config(config_path.to_str().unwrap());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Check that init failed
        assert!(result.is_err(), "init_config should fail when file exists");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("already exists"), "Error should mention file already exists: {}", err_msg);
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
        let result = init_config(config_path.to_str().unwrap());

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
}
