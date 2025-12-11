mod config;
mod log;
mod procfile;
mod process;
mod ui;

use config::Config;
use procfile::Procfile;
use process::ProcessManager;
use ui::App;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// Overitall - Process and log management TUI
#[derive(Parser, Debug)]
#[command(name = "overitall")]
#[command(about = "Process and log management TUI combining overmind + lnav", long_about = None)]
struct Cli {
    /// Path to config file (defaults to .overitall.toml)
    #[arg(short, long, default_value = ".overitall.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    let config_path = &cli.config;

    // Load config
    let mut config = Config::from_file(config_path)?;
    config.config_path = Some(std::path::PathBuf::from(config_path));

    // Parse procfile
    let procfile = Procfile::from_file(&config.procfile)?;

    // Create process manager
    let mut manager = ProcessManager::new();

    // Add processes from Procfile
    for (name, command) in &procfile.processes {
        manager.add_process(name.clone(), command.clone());

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

    // Return result
    result
}

/// Command parsed from user input
enum Command {
    Quit,
    Restart(String),
    Kill(String),
    FilterInclude(String),
    FilterExclude(String),
    FilterClear,
    FilterList,
    Unknown(String),
}

/// Parse a command from user input (without the leading ':')
fn parse_command(input: &str) -> Command {
    let input = input.trim();

    if input == "q" {
        return Command::Quit;
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Command::Unknown("Empty command".to_string());
    }

    match parts[0] {
        "r" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :r <process>".to_string())
            } else {
                Command::Restart(parts[1].to_string())
            }
        }
        "k" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :k <process>".to_string())
            } else {
                Command::Kill(parts[1].to_string())
            }
        }
        "f" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :f <text_or_regex>".to_string())
            } else {
                Command::FilterInclude(parts[1..].join(" "))
            }
        }
        "fn" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :fn <text_or_regex>".to_string())
            } else {
                Command::FilterExclude(parts[1..].join(" "))
            }
        }
        "fc" => Command::FilterClear,
        "fl" => Command::FilterList,
        _ => Command::Unknown(format!("Unknown command: {}", parts[0])),
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    manager: &mut ProcessManager,
    config: &mut Config,
) -> anyhow::Result<()> {
    loop {
        // Process logs from all sources
        manager.process_logs();

        // Draw UI
        terminal.draw(|f| {
            ui::draw(f, app, manager);
        })?;

        // Handle input with short timeout
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Ctrl-C always quits
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.quit();
                        break;
                    }
                    // Command mode specific keys
                    KeyCode::Char(':') if !app.command_mode && !app.search_mode => {
                        app.enter_command_mode();
                    }
                    KeyCode::Esc if app.command_mode => {
                        app.exit_command_mode();
                    }
                    // Search mode specific keys
                    KeyCode::Char('/') if !app.command_mode && !app.search_mode => {
                        app.enter_search_mode();
                    }
                    KeyCode::Esc if app.search_mode => {
                        app.exit_search_mode();
                    }
                    KeyCode::Enter if app.search_mode => {
                        let search_text = app.input.clone();
                        if !search_text.is_empty() {
                            app.perform_search(search_text);
                        }
                        app.exit_search_mode();
                    }
                    KeyCode::Backspace if app.search_mode => {
                        app.delete_char();
                    }
                    KeyCode::Char(c) if app.search_mode => {
                        app.add_char(c);
                    }
                    KeyCode::Enter if app.command_mode => {
                        let cmd_text = app.input.clone();
                        let cmd = parse_command(&cmd_text);

                        // Save to history before processing (don't save empty or quit commands)
                        if !cmd_text.trim().is_empty() && !matches!(cmd, Command::Quit) {
                            app.save_to_history(cmd_text);
                        }

                        match cmd {
                            Command::Quit => {
                                app.quit();
                                break;
                            }
                            Command::Restart(process) => {
                                match manager.restart_process(&process).await {
                                    Ok(_) => {
                                        app.set_status_success(format!("Restarted process: {}", process));
                                    }
                                    Err(e) => {
                                        app.set_status_error(format!("Failed to restart {}: {}", process, e));
                                    }
                                }
                            }
                            Command::Kill(process) => {
                                match manager.kill_process(&process).await {
                                    Ok(_) => {
                                        app.set_status_success(format!("Killed process: {}", process));
                                    }
                                    Err(e) => {
                                        app.set_status_error(format!("Failed to kill {}: {}", process, e));
                                    }
                                }
                            }
                            Command::FilterInclude(pattern) => {
                                app.add_include_filter(pattern.clone());
                                app.set_status_success(format!("Added include filter: {}", pattern));

                                // Save filters to config
                                config.update_filters(&app.filters);
                                if let Some(path) = &config.config_path {
                                    if let Err(e) = config.save(path.to_str().unwrap()) {
                                        eprintln!("Warning: failed to save filters: {}", e);
                                    }
                                }
                            }
                            Command::FilterExclude(pattern) => {
                                app.add_exclude_filter(pattern.clone());
                                app.set_status_success(format!("Added exclude filter: {}", pattern));

                                // Save filters to config
                                config.update_filters(&app.filters);
                                if let Some(path) = &config.config_path {
                                    if let Err(e) = config.save(path.to_str().unwrap()) {
                                        eprintln!("Warning: failed to save filters: {}", e);
                                    }
                                }
                            }
                            Command::FilterClear => {
                                let count = app.filter_count();
                                app.clear_filters();
                                app.set_status_success(format!("Cleared {} filter(s)", count));

                                // Save filters to config
                                config.update_filters(&app.filters);
                                if let Some(path) = &config.config_path {
                                    if let Err(e) = config.save(path.to_str().unwrap()) {
                                        eprintln!("Warning: failed to save filters: {}", e);
                                    }
                                }
                            }
                            Command::FilterList => {
                                if app.filters.is_empty() {
                                    app.set_status_info("No active filters".to_string());
                                } else {
                                    let filter_strs: Vec<String> = app.filters.iter().map(|f| {
                                        let type_str = match f.filter_type {
                                            ui::FilterType::Include => "include",
                                            ui::FilterType::Exclude => "exclude",
                                        };
                                        format!("{}: {}", type_str, f.pattern)
                                    }).collect();
                                    app.set_status_info(format!("Filters: {}", filter_strs.join(", ")));
                                }
                            }
                            Command::Unknown(msg) => {
                                app.set_status_error(format!("Error: {}", msg));
                            }
                        }
                        app.exit_command_mode();
                    }
                    KeyCode::Backspace if app.command_mode => {
                        app.delete_char();
                    }
                    KeyCode::Up if app.command_mode => {
                        app.history_prev();
                    }
                    KeyCode::Down if app.command_mode => {
                        app.history_next();
                    }
                    KeyCode::Char(c) if app.command_mode => {
                        app.add_char(c);
                    }
                    // Non-command mode keys
                    KeyCode::Char('q') if !app.command_mode && !app.search_mode => {
                        app.quit();
                        break;
                    }
                    // Search navigation (n/N for next/previous match)
                    KeyCode::Char('n') if !app.command_mode && !app.search_mode => {
                        // Get total matches from filtered logs
                        let logs = manager.get_all_logs();
                        let filtered_logs: Vec<_> = if app.filters.is_empty() {
                            logs
                        } else {
                            logs.into_iter()
                                .filter(|log| {
                                    let line_text = &log.line;
                                    for filter in &app.filters {
                                        if matches!(filter.filter_type, ui::FilterType::Exclude) {
                                            if filter.matches(line_text) {
                                                return false;
                                            }
                                        }
                                    }
                                    let include_filters: Vec<_> = app
                                        .filters
                                        .iter()
                                        .filter(|f| matches!(f.filter_type, ui::FilterType::Include))
                                        .collect();
                                    if include_filters.is_empty() {
                                        return true;
                                    }
                                    include_filters.iter().any(|filter| filter.matches(line_text))
                                })
                                .collect()
                        };
                        let total_matches = filtered_logs
                            .iter()
                            .filter(|log| {
                                log.line
                                    .to_lowercase()
                                    .contains(&app.search_pattern.to_lowercase())
                            })
                            .count();
                        app.next_match(total_matches);
                    }
                    KeyCode::Char('N') if !app.command_mode && !app.search_mode => {
                        // Get total matches from filtered logs
                        let logs = manager.get_all_logs();
                        let filtered_logs: Vec<_> = if app.filters.is_empty() {
                            logs
                        } else {
                            logs.into_iter()
                                .filter(|log| {
                                    let line_text = &log.line;
                                    for filter in &app.filters {
                                        if matches!(filter.filter_type, ui::FilterType::Exclude) {
                                            if filter.matches(line_text) {
                                                return false;
                                            }
                                        }
                                    }
                                    let include_filters: Vec<_> = app
                                        .filters
                                        .iter()
                                        .filter(|f| matches!(f.filter_type, ui::FilterType::Include))
                                        .collect();
                                    if include_filters.is_empty() {
                                        return true;
                                    }
                                    include_filters.iter().any(|filter| filter.matches(line_text))
                                })
                                .collect()
                        };
                        let total_matches = filtered_logs
                            .iter()
                            .filter(|log| {
                                log.line
                                    .to_lowercase()
                                    .contains(&app.search_pattern.to_lowercase())
                            })
                            .count();
                        app.prev_match(total_matches);
                    }
                    // Clear search with Esc (when not in command/search mode)
                    KeyCode::Esc if !app.command_mode && !app.search_mode => {
                        app.clear_search();
                    }
                    KeyCode::Up if !app.command_mode && !app.search_mode => {
                        app.scroll_up(1);
                    }
                    KeyCode::Down if !app.command_mode && !app.search_mode => {
                        let total_logs = manager.get_all_logs().len();
                        let max_offset = total_logs.saturating_sub(1);
                        app.scroll_down(1, max_offset);
                    }
                    KeyCode::PageUp if !app.command_mode && !app.search_mode => {
                        // Calculate page size (roughly 85% of terminal height - 2 for borders)
                        let term_size = terminal.size()?;
                        let log_area_height = (term_size.height * 85 / 100).saturating_sub(2) as usize;
                        app.scroll_up(log_area_height);
                    }
                    KeyCode::PageDown if !app.command_mode && !app.search_mode => {
                        let term_size = terminal.size()?;
                        let log_area_height = (term_size.height * 85 / 100).saturating_sub(2) as usize;
                        let total_logs = manager.get_all_logs().len();
                        let max_offset = total_logs.saturating_sub(1);
                        app.scroll_down(log_area_height, max_offset);
                    }
                    KeyCode::Home if !app.command_mode && !app.search_mode => {
                        app.scroll_to_top();
                    }
                    KeyCode::End if !app.command_mode && !app.search_mode => {
                        app.scroll_to_bottom();
                    }
                    _ => {}
                }
            }
        }

        // Small delay to prevent busy-looping
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
