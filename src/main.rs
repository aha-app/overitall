mod cli;
mod command;
mod config;
mod event_handler;
mod log;
mod procfile;
mod process;
mod ui;
mod updater;

use cli::{Cli, init_config};
use config::Config;
use event_handler::EventHandler;
use procfile::Procfile;
use process::ProcessManager;
use ui::App;

use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

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
                manager.add_log_file(name.clone(), log_file.clone()).await?;
            }
        }
    }

    // Start all processes
    manager.start_all().await?;

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

    // TUI event loop
    let result = run_app(&mut terminal, &mut app, &mut manager, &mut config).await;

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
) -> anyhow::Result<()> {
    let mut shutdown_ui_shown = false;
    let mut kill_signals_sent = false;

    loop {
        // Process logs from all sources
        manager.process_logs();

        // Draw UI
        terminal.draw(|f| {
            ui::draw(f, app, manager);
        })?;

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

        // Handle input with short timeout
        // Note: In raw mode, Ctrl+C is captured as a keyboard event, not a signal,
        // so we handle it in the event handler instead of using tokio::signal::ctrl_c()

        // Don't use tokio::select - check for input directly
        // This avoids potential issues with the tokio runtime
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let mut event_handler = EventHandler::new(app, manager, config);
                if event_handler.handle_key_event(key).await? {
                    break; // Quit was requested (shouldn't happen now)
                }
            }
        }

        // Small delay to prevent busy-looping
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

