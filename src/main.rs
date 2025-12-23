mod cli;
mod command;
mod config;
mod event_handler;
mod ipc;
mod log;
mod operations;
mod procfile;
mod process;
mod skill;
mod status_matcher;
mod traces;
mod ui;
mod updater;

use cli::{check_already_running, get_socket_path, Cli, Commands, VscodeAction, init_config, install_vscode_extension, run_ipc_command};
use config::Config;
use event_handler::EventHandler;
use ipc::state::{BufferStats, FilterInfo, LogLineInfo, ProcessInfo, StateSnapshot, ViewModeInfo};
use ipc::{IpcAction, IpcCommandHandler, IpcServer};
use procfile::Procfile;
use process::{ProcessManager, ProcessStatus};
use ui::{App, DisplayMode, FilterType};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, style::Color, Terminal};
use tokio::signal::unix::{signal, SignalKind};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    let config_path = &cli.config;

    // Handle --update flag: check for updates and exit
    if cli.update {
        match updater::check_and_update(VERSION) {
            Ok(()) => {
                println!("oit {} is up to date", VERSION);
            }
            Err(e) => {
                eprintln!("Error checking for updates: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

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
        return init_config(config_path, cli.procfile.as_deref());
    }

    // Handle vscode subcommand (doesn't need IPC)
    if let Some(Commands::Vscode { action }) = &cli.command {
        return match action {
            VscodeAction::Install => install_vscode_extension(),
        };
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

    // Check if another instance is already running in this directory
    if check_already_running().await? {
        eprintln!("Error: oit is already running in this directory.");
        eprintln!("Use 'oit ping' to verify, or remove .oit.sock if the previous instance crashed.");
        std::process::exit(1);
    }

    // Auto-install Claude/Cursor skill if .claude or .cursor directory exists
    skill::auto_install_skill();

    // Load config
    let mut config = Config::from_file(config_path)?;
    config.config_path = Some(std::path::PathBuf::from(config_path));

    // Use CLI-specified procfile as a temporary override (not saved to config)
    let runtime_procfile_path = cli.procfile
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| config.procfile.clone());

    // Parse procfile
    let procfile = Procfile::from_file(&runtime_procfile_path)?;

    // Validate config (check for name collisions between processes and log files)
    let process_names: Vec<String> = procfile.processes.keys().cloned().collect();
    config.validate(&process_names)?;

    // Determine working directory from Procfile path
    // If procfile is just "Procfile" (no directory), parent() returns Some("")
    // We need to filter out empty paths and use current_dir instead
    let procfile_dir = runtime_procfile_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Create process manager
    let max_buffer_mb = config.max_log_buffer_mb.unwrap_or(50);
    let mut manager = ProcessManager::new_with_buffer_limit(max_buffer_mb);

    // Add processes from Procfile
    for (name, command) in &procfile.processes {
        // Get status config if available
        let status_config = config.processes.get(name)
            .and_then(|pc| pc.status.as_ref());
        manager.add_process(name.clone(), command.clone(), Some(procfile_dir.clone()), status_config);

        // If this process has a log file configured, add it
        if let Some(proc_config) = config.processes.get(name) {
            if let Some(log_file) = &proc_config.log_file {
                let log_path = procfile_dir.join(log_file);
                manager.add_log_file(name.clone(), log_path).await?;
            }
        }
    }

    // Add standalone log files from config
    for log_file_config in &config.log_files {
        let log_path = procfile_dir.join(&log_file_config.path);
        manager.add_standalone_log_file(log_file_config.name.clone(), log_path).await?;
    }

    // Start all processes (collect failures, don't crash)
    let start_failures = manager.start_all().await;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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

    // Initialize process colors from config (only if enabled)
    if config.process_coloring == Some(true) {
        let process_names: Vec<String> = manager.get_processes().keys().cloned().collect();
        let log_file_names = manager.get_standalone_log_file_names();
        app.init_process_colors(&process_names, &log_file_names, &config.colors);
        app.coloring_enabled = true;
    }

    // Load display mode from config (default: Compact if not specified)
    // Config stores bool for backwards compat: true = Compact, false = Full
    if let Some(compact_mode) = config.compact_mode {
        if compact_mode {
            app.display_mode = DisplayMode::Compact;
        } else {
            app.display_mode = DisplayMode::Full;
        }
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
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;

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
    let mut headless_shutdown = false; // True when terminal is gone (SIGHUP)
    let ipc_handler = IpcCommandHandler::new(VERSION);

    // Set up signal handlers for graceful shutdown on SIGINT/SIGTERM/SIGHUP
    // SIGINT is typically Ctrl+C when not in raw mode, or sent via `kill -INT <pid>`
    // SIGTERM is sent by `kill <pid>` without arguments
    // SIGHUP is sent when the controlling terminal is closed (e.g., VS Code terminal)
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sighup = signal(SignalKind::hangup())?;

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
                    let snapshot = create_state_snapshot(app, manager);
                    let handler_result = ipc_handler.handle(&request, Some(&snapshot));

                    // Process any actions from the handler
                    for action in handler_result.actions {
                        apply_ipc_action(app, config, manager, action).await;
                    }

                    let _ = server.send_response(conn_id, handler_result.response).await;
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

        // Draw UI (skip if terminal is gone during headless shutdown)
        if !headless_shutdown {
            terminal.draw(|f| {
                ui::draw(f, app, manager);
            })?;
        }

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
            // In headless mode (SIGHUP), skip the UI drawing step and send signals immediately
            if headless_shutdown || shutdown_ui_shown {
                // Send kill signals if we haven't already
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
            _ = sighup.recv() => {
                // SIGHUP received (e.g., terminal closed)
                // Use headless shutdown since the terminal is gone
                headless_shutdown = true;
                app.start_shutdown();
            }
            // Handle terminal events (keyboard, mouse, resize)
            maybe_event = event_stream.next() => {
                if let Some(Ok(event)) = maybe_event {
                    match event {
                        Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                                let mut event_handler = EventHandler::new(app, manager, config);
                                if event_handler.handle_key_event(key).await? {
                                    return Ok(()); // Quit was requested
                                }
                            }
                        }
                        Event::Mouse(mouse) => {
                            let mut event_handler = EventHandler::new(app, manager, config);
                            event_handler.handle_mouse_event(mouse)?;
                        }
                        _ => {}
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

/// Convert a ratatui Color to a string name
fn color_to_string(color: Color) -> String {
    match color {
        Color::Green => "green",
        Color::Yellow => "yellow",
        Color::Red => "red",
        Color::Blue => "blue",
        Color::Cyan => "cyan",
        Color::Magenta => "magenta",
        Color::White => "white",
        Color::Gray => "gray",
        _ => "white",
    }
    .to_string()
}

/// Create a StateSnapshot from current App and ProcessManager state for IPC commands
fn create_state_snapshot(app: &App, manager: &ProcessManager) -> StateSnapshot {
    // Build process info list
    let processes: Vec<ProcessInfo> = manager
        .get_processes()
        .iter()
        .map(|(name, handle)| {
            let (status, error) = match &handle.status {
                ProcessStatus::Running => ("running".to_string(), None),
                ProcessStatus::Stopped => ("stopped".to_string(), None),
                ProcessStatus::Terminating => ("terminating".to_string(), None),
                ProcessStatus::Restarting => ("restarting".to_string(), None),
                ProcessStatus::Failed(msg) => ("failed".to_string(), Some(msg.clone())),
            };
            let (custom_label, custom_color) = handle
                .get_custom_status()
                .map(|(label, color)| {
                    let color_str = color.map(|c| color_to_string(c));
                    (Some(label.to_string()), color_str)
                })
                .unwrap_or((None, None));
            ProcessInfo {
                name: name.clone(),
                status,
                error,
                custom_label,
                custom_color,
            }
        })
        .collect();

    // Build filter info list
    let active_filters: Vec<FilterInfo> = app
        .filters
        .iter()
        .map(|f| FilterInfo {
            pattern: f.pattern.clone(),
            filter_type: match f.filter_type {
                FilterType::Include => "include".to_string(),
                FilterType::Exclude => "exclude".to_string(),
            },
        })
        .collect();

    // Get buffer stats
    let stats = manager.get_buffer_stats();
    let buffer_stats = BufferStats {
        buffer_bytes: (stats.memory_mb * 1024.0 * 1024.0) as usize,
        max_buffer_bytes: stats.limit_mb * 1024 * 1024,
        usage_percent: stats.percent,
    };

    // Get recent logs (last 1000 for IPC - callers can use limit/offset)
    let recent_logs: Vec<LogLineInfo> = manager
        .get_recent_logs(1000)
        .iter()
        .map(|log| LogLineInfo {
            id: log.id,
            process: log.source.process_name().to_string(),
            content: log.line.clone(),
            timestamp: log.timestamp.to_rfc3339(),
            batch_id: None, // Batch detection is expensive; skip for now
        })
        .collect();

    let total_log_lines = stats.line_count;

    StateSnapshot {
        processes,
        log_files: manager.get_standalone_log_file_names(),
        filter_count: app.filters.len(),
        active_filters,
        search_pattern: if app.search_pattern.is_empty() {
            None
        } else {
            Some(app.search_pattern.clone())
        },
        view_mode: ViewModeInfo {
            frozen: app.frozen,
            batch_view: app.batch_view_mode,
            trace_filter: app.trace_filter_mode,
            trace_selection: app.trace_selection_mode,
            display_mode: app.display_mode.name().to_string(),
        },
        auto_scroll: app.auto_scroll,
        log_count: stats.line_count,
        buffer_stats,
        trace_recording: app.manual_trace_recording,
        active_trace_id: app.active_trace_id.clone(),
        recent_logs,
        total_log_lines,
        hidden_processes: app.hidden_processes.iter().cloned().collect(),
    }
}

/// Apply an IPC action to the App state
async fn apply_ipc_action(
    app: &mut App,
    config: &mut Config,
    manager: &mut ProcessManager,
    action: IpcAction,
) {
    match action {
        IpcAction::SetSearch { pattern } => {
            app.perform_search(pattern);
        }
        IpcAction::ClearSearch => {
            app.clear_search();
        }
        IpcAction::SetAutoScroll { enabled } => {
            app.auto_scroll = enabled;
        }
        IpcAction::SelectAndExpandLine { id } => {
            app.selected_line_id = Some(id);
            app.expanded_line_view = true;
        }
        IpcAction::ScrollToLine { id } => {
            // Set the selected line - log_viewer will auto-scroll to show it
            app.selected_line_id = Some(id);
        }
        IpcAction::ScrollUp { lines } => {
            app.scroll_up(lines);
        }
        IpcAction::ScrollDown { lines } => {
            // Use saturating_add since we don't have max_offset here
            // The log_viewer will clamp during rendering
            app.scroll_offset = app.scroll_offset.saturating_add(lines);
        }
        IpcAction::ScrollToTop => {
            app.scroll_to_top();
        }
        IpcAction::SetFrozen { frozen } => {
            if frozen {
                app.freeze_display();
            } else {
                app.unfreeze_display();
            }
        }
        IpcAction::AddFilter { pattern, is_exclude } => {
            if is_exclude {
                operations::filter::add_exclude_filter(app, config, pattern);
            } else {
                operations::filter::add_include_filter(app, config, pattern);
            }
        }
        IpcAction::RemoveFilter { pattern } => {
            operations::filter::remove_filter(app, config, &pattern);
        }
        IpcAction::ClearFilters => {
            operations::filter::clear_filters(app, config);
        }
        IpcAction::HideProcess { name } => {
            // Runtime only - directly modify hidden_processes without saving to config
            app.hidden_processes.insert(name);
        }
        IpcAction::ShowProcess { name } => {
            // Runtime only - directly modify hidden_processes without saving to config
            app.hidden_processes.remove(&name);
        }
        IpcAction::RestartProcess { name } => {
            // Non-blocking: set restart flag, main loop handles actual restart
            if manager.set_restarting(&name) {
                app.set_status_info(format!("Restarting: {}", name));
            } else {
                app.set_status_error(format!("Process not found: {}", name));
            }
        }
        IpcAction::RestartAllProcesses => {
            // Non-blocking: set restart flags for all processes
            let names: Vec<String> = manager
                .get_all_statuses()
                .into_iter()
                .map(|(n, _)| n)
                .collect();
            if names.is_empty() {
                app.set_status_error("No processes to restart".to_string());
            } else {
                manager.set_all_restarting();
                app.set_status_info(format!("Restarting {} process(es)...", names.len()));
            }
        }
        IpcAction::KillProcess { name } => {
            match operations::process::kill_process(manager, &name).await {
                Ok(msg) => app.set_status_success(msg),
                Err(msg) => app.set_status_error(msg),
            }
        }
        IpcAction::StartProcess { name } => {
            match operations::process::start_process(manager, &name).await {
                Ok(msg) => app.set_status_success(msg),
                Err(msg) => app.set_status_error(msg),
            }
        }
    }
}

