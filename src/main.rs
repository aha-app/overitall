mod cli;
mod command;
mod config;
mod event_handler;
mod log;
mod procfile;
mod process;
mod ui;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    let config_path = &cli.config;

    // Handle --init flag
    if cli.init {
        return init_config(config_path);
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
    let mut manager = ProcessManager::new();

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

    loop {
        // Process logs from all sources
        manager.process_logs();

        // Draw UI
        terminal.draw(|f| {
            ui::draw(f, app, manager);
        })?;

        // Check if we're shutting down and all processes have terminated
        // Only check after we've shown at least one frame of shutdown UI
        if app.shutting_down {
            if shutdown_ui_shown {
                if manager.check_termination_status().await {
                    // All processes terminated, we can exit now
                    app.quit();
                    break;
                }
            } else {
                // Mark that we've shown the shutdown UI at least once
                shutdown_ui_shown = true;
            }
        }

        // Handle input with short timeout
        // Note: In raw mode, Ctrl+C is captured as a keyboard event, not a signal,
        // so we handle it in the event handler instead of using tokio::signal::ctrl_c()
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                // Check for keyboard input
                if event::poll(std::time::Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        let mut event_handler = EventHandler::new(app, manager, config);
                        if event_handler.handle_key_event(key).await? {
                            break; // Quit was requested (shouldn't happen now)
                        }
                    }
                }
            }
        }

        // Small delay to prevent busy-looping
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

