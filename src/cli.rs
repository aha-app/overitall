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

    /// Path to Procfile (overrides config file setting)
    #[arg(short = 'f', long = "file")]
    pub procfile: Option<String>,

    /// Initialize a new .overitall.toml config file from Procfile
    #[arg(long)]
    pub init: bool,

    /// Skip auto-update check on startup
    #[arg(long)]
    pub no_update: bool,

    /// Check for updates and exit (doesn't start processes)
    #[arg(long)]
    pub update: bool,

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
    /// VS Code extension management
    Vscode {
        #[command(subcommand)]
        action: VscodeAction,
    },
}

/// VS Code extension subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum VscodeAction {
    /// Install the VS Code extension from GitHub releases
    Install,
}

const REPO: &str = "jemmyw/overitall";
const VSIX_PATTERN: &str = "vscode-overitall-*.vsix";

/// Install the VS Code extension from GitHub releases
pub fn install_vscode_extension() -> anyhow::Result<()> {
    use std::process::{Command, Stdio};

    // Check if gh CLI is available
    let gh_status = Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if !gh_status.map(|s| s.success()).unwrap_or(false) {
        return Err(anyhow!(
            "gh CLI not found or not authenticated.\n\
             Install gh and run: gh auth login"
        ));
    }

    // Check if VS Code CLI is available
    let code_status = Command::new("code")
        .args(["--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if !code_status.map(|s| s.success()).unwrap_or(false) {
        return Err(anyhow!(
            "VS Code CLI 'code' not found.\n\
             Install VS Code and ensure 'code' is in your PATH.\n\
             In VS Code: Cmd+Shift+P > 'Shell Command: Install code command in PATH'"
        ));
    }

    // Get the latest release tag
    println!("Checking for latest release...");
    let tag_output = Command::new("gh")
        .args([
            "release",
            "view",
            "--repo",
            REPO,
            "--json",
            "tagName",
            "-q",
            ".tagName",
        ])
        .output()?;

    if !tag_output.status.success() {
        let stderr = String::from_utf8_lossy(&tag_output.stderr);
        return Err(anyhow!("Failed to get latest release: {}", stderr));
    }

    let tag = String::from_utf8(tag_output.stdout)?
        .trim()
        .to_string();

    let version = tag.trim_start_matches('v');
    println!("Latest release: {} ({})", tag, version);

    // Create temp directory and download the VSIX
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    println!("Downloading VS Code extension...");
    let download_status = Command::new("gh")
        .args([
            "release",
            "download",
            &tag,
            "--repo",
            REPO,
            "--pattern",
            VSIX_PATTERN,
            "--dir",
            temp_path.to_str().unwrap(),
        ])
        .status()?;

    if !download_status.success() {
        return Err(anyhow!("Failed to download extension"));
    }

    // Find the downloaded VSIX file
    let vsix_file = std::fs::read_dir(temp_path)?
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with(".vsix")
        })
        .ok_or_else(|| anyhow!("VSIX file not found in download"))?;

    let vsix_path = vsix_file.path();
    println!("Downloaded: {}", vsix_file.file_name().to_string_lossy());

    // Install the extension
    println!("Installing extension...");
    let install_status = Command::new("code")
        .args([
            "--install-extension",
            vsix_path.to_str().unwrap(),
            "--force",
        ])
        .status()?;

    if !install_status.success() {
        return Err(anyhow!("Failed to install extension"));
    }

    println!("\n✓ VS Code extension installed successfully!");
    println!("\nThe Overitall extension is now available in VS Code.");
    println!("Look for the Overitall icon in the activity bar when you have a Procfile.");

    Ok(())
}

/// Initialize a new config file from an existing Procfile
pub fn init_config(config_path: &str, procfile_override: Option<&str>) -> anyhow::Result<()> {
    let config_exists = Path::new(config_path).exists();

    // Only create config if it doesn't exist
    if !config_exists {
        // Use override or default Procfile location
        let procfile_path = procfile_override.unwrap_or("Procfile");

        // Check if Procfile exists and provide helpful error if not
        if !Path::new(procfile_path).exists() {
            return Err(anyhow!(
                "Procfile not found at '{}'.\n\n\
                To use --init, first create a Procfile with your processes.\n\
                Example Procfile:\n\
                \n\
                  web: rails server -p 3000\n\
                  worker: bundle exec sidekiq\n\
                \n\
                See: https://devcenter.heroku.com/articles/procfile\n\
                \n\
                Then run 'oit --init' again to generate the config file.\n\
                Or specify a Procfile with: oit --init -f <path>",
                procfile_path
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
                    status: None,
                },
            );
        }

        let config = Config {
            procfile: std::path::PathBuf::from(procfile_path),
            processes,
            log_files: Vec::new(),
            filters: config::FilterConfig {
                include: vec![],
                exclude: vec![],
            },
            batch_window_ms: Some(100),
            max_log_buffer_mb: Some(50),
            hidden_processes: Vec::new(),
            disable_auto_update: None,
            compact_mode: None,
            colors: std::collections::HashMap::new(),
            process_coloring: None,
            config_path: None,
        };

        // Save the config
        config.save(config_path)
            .with_context(|| format!("Failed to write config to '{}'", config_path))?;

        // Append commented-out process_coloring option
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .append(true)
            .open(config_path)
            .with_context(|| format!("Failed to append to '{}'", config_path))?;
        writeln!(file, "\n# Enable colored process names (disabled by default)")?;
        writeln!(file, "# process_coloring = true")?;

        // Print success message
        println!("Created {} with {} processes:", config_path, process_names.len());
        for name in &process_names {
            println!("  - {}", name);
        }

        println!("\nNext steps:");
        println!("  1. Edit {} to configure log file paths", config_path);
        println!("  2. Run 'oit' to start the TUI");
    } else {
        println!("Config file '{}' already exists.", config_path);
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
        Commands::Vscode { .. } => {
            // This should be handled separately in main.rs, not via IPC
            return Err(anyhow!("vscode commands don't use IPC"));
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
#[path = "cli_tests.rs"]
mod tests;
