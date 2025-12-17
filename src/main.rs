mod cli;
mod command;
mod config;
mod event_handler;
mod ipc;
mod log;
mod operations;
mod procfile;
mod process;
mod traces;
mod ui;
mod updater;

use cli::{get_socket_path, Cli, Commands, init_config, run_ipc_command};
use config::Config;
use event_handler::EventHandler;
use ipc::{IpcCommandHandler, IpcServer};
use procfile::Procfile;
use process::ProcessManager;
use ui::App;

use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::signal::unix::{signal, SignalKind};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    let config_path = &cli.config;

    // Check for updates (unless disabled via --no-update or config file)
    // If update succeeds, this will re-exec and never return
    let config_disables_update = Config::from_file(config_path)
        .map(|c| c.disable_auto_update.unwrap_or(false))
        .unwrap_or(false);
    if !cli.no_update && !config_disables_update {
        if let Err(e) = updater::check_and_update(VERSION) {
            eprintln!("Warning: Could not check for updates: {}", e);
        }
    }

    // Handle --init flag
    if cli.init {
        return init_config(config_path);
    }

    // Handle IPC subcommands (ping, status, etc.)
    // These communicate with a running TUI instance and exit
    if let Some(ref command) = cli.command {
        return run_ipc_command(command).await;
    }

    // Check if config file exists and provide helpful error if not
    if !std::path::Path::new(config_path).exists() {
        eprintln!("Error: Config file '{}' not found.\n", config_path);
        eprintln!("To get started:");
        eprintln!("  1. Create a Procfile with your processes (e.g., 'web: rails server')");
        eprintln!("  2. Run 'oit --init' to generate a config file");
        eprintln!("  3. Run 'oit' to start the TUI\n");
        eprintln!("Or specify a config file with: oit --config <path>\n");
        eprintln!("For more help, run: oit --help");
        std::process::exit(1);
    }

    // Load config
    let mut config = Config::from_file(config_path)?;
    config.config_path = Some(std::path::PathBuf::from(config_path));

    // Parse procfile
    let procfile = Procfile::from_file(&config.procfile)?;

    // Determine working directory from Procfile path
    // If procfile is just "Procfile" (no directory), parent() returns Some("")
    // We need to filter out empty paths and use current_dir instead
    let procfile_dir = config.procfile
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Create process manager
    let max_buffer_mb = config.max_log_buffer_mb.unwrap_or(50);
    let mut manager = ProcessManager::new_with_buffer_limit(max_buffer_mb);

    // Add processes from Procfile
    for (name, command) in &procfile.processes {
        manager.add_process(name.clone(), command.clone(), Some(procfile_dir.clone()));

        // If this process has a log file configured, add it
        if let Some(proc_config) = config.processes.get(name) {
            if let Some(log_file) = &proc_config.log_file {
                let log_path = procfile_dir.join(log_file);
                manager.add_log_file(name.clone(), log_path).await?;
            }
        }
    }

    // Start all processes (collect failures, don't crash)
    let start_failures = manager.start_all().await;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Load batch window from config if specified
    if let Some(batch_window_ms) = config.batch_window_ms {
        app.set_batch_window(batch_window_ms);
    }

    // Load filters from config
    for pattern in &config.filters.include {
        app.add_include_filter(pattern.clone());
    }
    for pattern in &config.filters.exclude {
        app.add_exclude_filter(pattern.clone());
    }

    // Load hidden processes from config
    app.hidden_processes = config.hidden_processes.iter().cloned().collect();

    // Load compact mode from config (default: true if not specified)
    if let Some(compact_mode) = config.compact_mode {
        app.compact_mode = compact_mode;
    }

    // Show startup failures in status bar
    if !start_failures.is_empty() {
        let failure_names: Vec<&str> = start_failures.iter().map(|(n, _)| n.as_str()).collect();
        app.set_status_error(format!("Failed to start: {}", failure_names.join(", ")));
    }

    // Create IPC server for remote control
    let socket_path = get_socket_path();
    let mut ipc_server = match IpcServer::new(&socket_path) {
        Ok(server) => Some(server),
        Err(e) => {
            eprintln!("Warning: Could not create IPC server at {:?}: {}", socket_path, e);
            None
        }
    };

    // TUI event loop
    let result = run_app(&mut terminal, &mut app, &mut manager, &mut config, &mut ipc_server).await;

    // Cleanup IPC socket
    if let Some(ref server) = ipc_server {
        let _ = server.cleanup();
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Kill all processes before exiting
    manager.kill_all().await?;

    // Return result
    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    manager: &mut ProcessManager,
    config: &mut Config,
    ipc_server: &mut Option<IpcServer>,
) -> anyhow::Result<()> {
    let mut shutdown_ui_shown = false;
    let mut kill_signals_sent = false;
    let ipc_handler = IpcCommandHandler::new(VERSION);

    // Set up signal handlers for graceful shutdown on SIGINT/SIGTERM
    // SIGINT is typically Ctrl+C when not in raw mode, or sent via `kill -INT <pid>`
    // SIGTERM is sent by `kill <pid>` without arguments
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    // Create async event stream for terminal events
    let mut event_stream = EventStream::new();

    loop {
        // Process logs from all sources
        manager.process_logs();

        // Handle IPC requests from CLI clients
        if let Some(server) = ipc_server.as_mut() {
            // Accept any pending new connections
            let _ = server.accept_pending();

            // Poll for incoming commands
            if let Ok(requests) = server.poll_commands() {
                for (conn_id, request) in requests {
                    let response = ipc_handler.handle(&request, None);
                    let _ = server.send_response(conn_id, response).await;
                }
            }
        }

        // Check for newly failed processes (not during shutdown)
        if !app.shutting_down {
            let newly_failed = manager.check_all_status().await;
            if !newly_failed.is_empty() {
                // Show the first failure in status bar (to avoid overwhelming)
                let (name, msg) = &newly_failed[0];
                app.set_status_error(format!("{}: {}", name, msg));
            }
        }

        // Draw UI
        terminal.draw(|f| {
            ui::draw(f, app, manager);
        })?;

        // Handle pending restarts (after UI has been drawn showing "Restarting" status)
        if manager.has_pending_restarts() {
            let (succeeded, failed) = manager.perform_pending_restarts().await;

            // Update status bar with result
            if !failed.is_empty() {
                let failed_names: Vec<&str> = failed.iter().map(|(n, _)| n.as_str()).collect();
                app.set_status_error(format!("Restart failed: {}", failed_names.join(", ")));
            } else if !succeeded.is_empty() {
                app.set_status_success(format!("Restarted: {}", succeeded.join(", ")));
            }
        }

        // Check if we're shutting down
        if app.shutting_down {
            if shutdown_ui_shown {
                // UI has been drawn with "Terminating" status
                // Now send kill signals if we haven't already
                if !kill_signals_sent {
                    manager.send_kill_signals().await?;
                    kill_signals_sent = true;
                }

                // Check if all processes have terminated
                if manager.check_termination_status().await {
                    app.quit();
                    break;
                }
            } else {
                // Mark that we've shown the shutdown UI at least once
                // Next iteration will send kill signals
                shutdown_ui_shown = true;
            }
        }

        // Check if quit was called directly (shouldn't happen with graceful shutdown)
        if app.should_quit {
            break;
        }

        // Use tokio::select! to handle terminal events, signals, and timeouts concurrently
        // This ensures we can respond to SIGINT/SIGTERM even while waiting for terminal input
        tokio::select! {
            // Handle Unix signals for graceful shutdown
            _ = sigint.recv() => {
                // SIGINT received (e.g., kill -INT <pid>)
                // Note: In raw terminal mode, Ctrl+C is captured as a keyboard event,
                // so this branch handles external SIGINT only
                app.start_shutdown();
            }
            _ = sigterm.recv() => {
                // SIGTERM received (e.g., kill <pid>)
                app.start_shutdown();
            }
            // Handle terminal events (keyboard, mouse, resize)
            maybe_event = event_stream.next() => {
                if let Some(Ok(event)) = maybe_event {
                    // Only process key press events (not release/repeat)
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            let mut event_handler = EventHandler::new(app, manager, config);
                            if event_handler.handle_key_event(key).await? {
                                return Ok(()); // Quit was requested
                            }
                        }
                    }
                }
            }
            // Timeout for periodic updates (log processing, IPC polling)
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {
                // Just continue to process logs and redraw
            }
        }
    }

    Ok(())
}

