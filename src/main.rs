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

    // Return result
    result
}

/// Command parsed from user input
enum Command {
    Quit,
    Start(String),
    Restart(String),
    Kill(String),
    FilterInclude(String),
    FilterExclude(String),
    FilterClear,
    FilterList,
    NextBatch,
    PrevBatch,
    ShowBatch,
    SetBatchWindow(i64),
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
        "s" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :s <process>".to_string())
            } else {
                Command::Start(parts[1].to_string())
            }
        }
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
        "nb" => Command::NextBatch,
        "pb" => Command::PrevBatch,
        "sb" => Command::ShowBatch,
        "bw" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :bw <milliseconds>".to_string())
            } else {
                match parts[1].parse::<i64>() {
                    Ok(ms) if ms > 0 => Command::SetBatchWindow(ms),
                    Ok(_) => Command::Unknown("Batch window must be positive".to_string()),
                    Err(_) => Command::Unknown("Batch window must be a valid number".to_string()),
                }
            }
        }
        _ => Command::Unknown(format!("Unknown command: {}", parts[0])),
    }
}

/// Apply filters to a vector of log references, returning owned logs that pass all filters
fn apply_filters(logs: Vec<&log::LogLine>, filters: &[ui::Filter]) -> Vec<log::LogLine> {
    if filters.is_empty() {
        return logs.into_iter().map(|log| (*log).clone()).collect();
    }

    logs.into_iter()
        .filter(|log| {
            let line_text = &log.line;
            // First, check exclude filters - if any match, exclude the log
            for filter in filters {
                if matches!(filter.filter_type, ui::FilterType::Exclude) {
                    if filter.matches(line_text) {
                        return false;
                    }
                }
            }
            // Then, check include filters - if there are any, at least one must match
            let include_filters: Vec<_> = filters
                .iter()
                .filter(|f| matches!(f.filter_type, ui::FilterType::Include))
                .collect();
            if include_filters.is_empty() {
                return true;
            }
            include_filters.iter().any(|filter| filter.matches(line_text))
        })
        .map(|log| (*log).clone())
        .collect()
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
                    KeyCode::Char(':') if !app.command_mode && !app.search_mode && !app.show_help => {
                        app.enter_command_mode();
                    }
                    KeyCode::Esc if app.show_help => {
                        app.toggle_help();
                    }
                    KeyCode::Esc if app.expanded_line_view => {
                        app.close_expanded_view();
                    }
                    KeyCode::Esc if app.command_mode => {
                        app.exit_command_mode();
                    }
                    // Search mode specific keys
                    KeyCode::Char('/') if !app.command_mode && !app.search_mode && !app.show_help => {
                        app.enter_search_mode();
                    }
                    KeyCode::Esc if app.search_mode => {
                        app.exit_search_mode();
                    }
                    KeyCode::Enter if app.expanded_line_view => {
                        // Close expanded view with Enter
                        app.close_expanded_view();
                    }
                    KeyCode::Enter if app.search_mode => {
                        let search_text = app.input.clone();
                        if !search_text.is_empty() {
                            app.perform_search(search_text);
                        }
                        app.exit_search_mode();
                    }
                    KeyCode::Enter if !app.command_mode && !app.search_mode && !app.expanded_line_view => {
                        // Toggle expanded view for selected line
                        if app.selected_line_index.is_some() {
                            app.toggle_expanded_view();
                        }
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
                            Command::Start(process) => {
                                match manager.start_process(&process).await {
                                    Ok(_) => {
                                        app.set_status_success(format!("Started process: {}", process));
                                    }
                                    Err(e) => {
                                        app.set_status_error(format!("Failed to start {}: {}", process, e));
                                    }
                                }
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
                            Command::NextBatch => {
                                app.next_batch();
                                app.set_status_info("Next batch".to_string());
                            }
                            Command::PrevBatch => {
                                app.prev_batch();
                                app.set_status_info("Previous batch".to_string());
                            }
                            Command::ShowBatch => {
                                app.toggle_batch_view();
                                if app.batch_view_mode {
                                    app.set_status_info("Batch view mode enabled".to_string());
                                } else {
                                    app.set_status_info("Batch view mode disabled".to_string());
                                }
                            }
                            Command::SetBatchWindow(ms) => {
                                app.set_batch_window(ms);
                                // Count batches with the new window to show in status
                                let logs = manager.get_all_logs();
                                let filtered_logs = apply_filters(logs, &app.filters);
                                let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
                                let batches = ui::detect_batches_from_logs(&filtered_refs, ms);
                                app.set_status_success(format!("Batch window set to {}ms ({} batches detected)", ms, batches.len()));

                                // Save to config
                                config.batch_window_ms = Some(ms);
                                if let Some(config_path) = &config.config_path {
                                    if let Err(e) = config.save_to_file(config_path) {
                                        eprintln!("Warning: Failed to save config: {}", e);
                                    }
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
                    KeyCode::Char('?') if !app.command_mode && !app.search_mode => {
                        app.toggle_help();
                    }
                    // Search navigation (n/N for next/previous match)
                    KeyCode::Char('n') if !app.command_mode && !app.search_mode => {
                        // Get total matches from filtered logs
                        let logs = manager.get_all_logs();
                        let filtered_logs = apply_filters(logs, &app.filters);

                        // Build search matches vector
                        let search_matches: Vec<usize> = filtered_logs
                            .iter()
                            .enumerate()
                            .filter(|(_, log)| {
                                log.line
                                    .to_lowercase()
                                    .contains(&app.search_pattern.to_lowercase())
                            })
                            .map(|(idx, _)| idx)
                            .collect();

                        let total_matches = search_matches.len();
                        app.next_match(total_matches);

                        // Set selected line to the current match
                        if let Some(match_idx) = app.current_match {
                            if match_idx < search_matches.len() {
                                app.selected_line_index = Some(search_matches[match_idx]);
                            }
                        }
                    }
                    KeyCode::Char('N') if !app.command_mode && !app.search_mode => {
                        // Get total matches from filtered logs
                        let logs = manager.get_all_logs();
                        let filtered_logs = apply_filters(logs, &app.filters);

                        // Build search matches vector
                        let search_matches: Vec<usize> = filtered_logs
                            .iter()
                            .enumerate()
                            .filter(|(_, log)| {
                                log.line
                                    .to_lowercase()
                                    .contains(&app.search_pattern.to_lowercase())
                            })
                            .map(|(idx, _)| idx)
                            .collect();

                        let total_matches = search_matches.len();
                        app.prev_match(total_matches);

                        // Set selected line to the current match
                        if let Some(match_idx) = app.current_match {
                            if match_idx < search_matches.len() {
                                app.selected_line_index = Some(search_matches[match_idx]);
                            }
                        }
                    }
                    // Batch navigation with [ and ] keys
                    KeyCode::Char('[') if !app.command_mode && !app.search_mode => {
                        app.prev_batch();
                    }
                    KeyCode::Char(']') if !app.command_mode && !app.search_mode => {
                        app.next_batch();
                    }
                    // Clear search with Esc (when not in command/search mode)
                    KeyCode::Esc if !app.command_mode && !app.search_mode => {
                        app.clear_search();
                    }
                    // Copy selected line to clipboard
                    KeyCode::Char('c') if !app.command_mode && !app.search_mode && !app.expanded_line_view => {
                        if let Some(line_idx) = app.selected_line_index {
                            // Get all logs and apply filters (same logic as in draw_expanded_line_overlay)
                            let logs = manager.get_all_logs();
                            let filtered_logs = apply_filters(logs, &app.filters);

                            // Apply batch view mode filtering if enabled
                            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
                            let batches = ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);
                            let display_logs: Vec<_> = if app.batch_view_mode {
                                if let Some(batch_idx) = app.current_batch {
                                    if !batches.is_empty() && batch_idx < batches.len() {
                                        let (start, end) = batches[batch_idx];
                                        filtered_logs[start..=end].to_vec()
                                    } else {
                                        filtered_logs
                                    }
                                } else {
                                    filtered_logs
                                }
                            } else {
                                filtered_logs
                            };

                            if line_idx < display_logs.len() {
                                let log = &display_logs[line_idx];
                                let formatted = format!(
                                    "[{}] {}: {}",
                                    log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                    log.source.process_name(),
                                    log.line
                                );

                                match overitall::clipboard::copy_to_clipboard(&formatted) {
                                    Ok(_) => app.set_status_success("Copied line to clipboard".to_string()),
                                    Err(e) => app.set_status_error(format!("Failed to copy: {}", e)),
                                }
                            }
                        }
                    }
                    // Copy entire batch to clipboard
                    KeyCode::Char('C') if !app.command_mode && !app.search_mode && !app.expanded_line_view => {
                        if let Some(line_idx) = app.selected_line_index {
                            // Get all logs and apply filters
                            let logs = manager.get_all_logs();
                            let filtered_logs = apply_filters(logs, &app.filters);

                            // Detect batches
                            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
                            let batches = ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);

                            // Find which batch contains the selected line
                            if let Some((batch_idx, (start, end))) = batches.iter().enumerate().find(|(_, (start, end))| {
                                line_idx >= *start && line_idx <= *end
                            }) {
                                // Format the entire batch
                                let mut batch_text = format!("=== Batch {} ({} lines) ===\n", batch_idx + 1, end - start + 1);

                                for log in &filtered_logs[*start..=*end] {
                                    batch_text.push_str(&format!(
                                        "[{}] {}: {}\n",
                                        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                        log.source.process_name(),
                                        log.line
                                    ));
                                }

                                match overitall::clipboard::copy_to_clipboard(&batch_text) {
                                    Ok(_) => app.set_status_success(format!("Copied batch to clipboard ({} lines)", end - start + 1)),
                                    Err(e) => app.set_status_error(format!("Failed to copy: {}", e)),
                                }
                            } else {
                                app.set_status_error("No batch found for selected line".to_string());
                            }
                        }
                    }
                    // Focus on batch containing selected line
                    KeyCode::Char('b') if !app.command_mode && !app.search_mode && !app.expanded_line_view => {
                        if let Some(line_idx) = app.selected_line_index {
                            // Get all logs and apply filters
                            let logs = manager.get_all_logs();
                            let filtered_logs = apply_filters(logs, &app.filters);

                            // Detect batches
                            let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
                            let batches = ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);

                            // Find which batch contains the selected line
                            if let Some((batch_idx, _)) = batches.iter().enumerate().find(|(_, (start, end))| {
                                line_idx >= *start && line_idx <= *end
                            }) {
                                app.current_batch = Some(batch_idx);
                                app.batch_view_mode = true;
                                app.scroll_offset = 0; // Reset scroll to show batch from the start
                                app.set_status_info(format!("Focused on batch {}", batch_idx + 1));
                            } else {
                                app.set_status_error("No batch found for selected line".to_string());
                            }
                        }
                    }
                    KeyCode::Up if !app.command_mode && !app.search_mode => {
                        // Line selection: select previous line
                        app.select_prev_line();
                    }
                    KeyCode::Down if !app.command_mode && !app.search_mode => {
                        // Line selection: select next line
                        // Calculate the correct max based on filtered logs and batch view mode
                        let logs = manager.get_all_logs();
                        let filtered_logs = apply_filters(logs, &app.filters);

                        // If in batch view mode, limit to current batch
                        let filtered_refs: Vec<&log::LogLine> = filtered_logs.iter().collect();
                        let total_logs = if app.batch_view_mode {
                            if let Some(batch_idx) = app.current_batch {
                                let batches = ui::detect_batches_from_logs(&filtered_refs, app.batch_window_ms);
                                if !batches.is_empty() && batch_idx < batches.len() {
                                    let (start, end) = batches[batch_idx];
                                    end - start + 1
                                } else {
                                    filtered_logs.len()
                                }
                            } else {
                                filtered_logs.len()
                            }
                        } else {
                            filtered_logs.len()
                        };

                        app.select_next_line(total_logs);
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
