use crate::config::Config;
use crate::operations::{batch, batch_window, filter};
use crate::process::ProcessManager;
use crate::ui::App;
use anyhow::Result;

/// Command parsed from user input
#[derive(Debug, PartialEq)]
pub enum Command {
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
    ShowBatchWindow,
    Hide(String),
    Show(String),
    HideAll,
    ShowAll,
    Unknown(String),
}

/// Parse a command from user input (without the leading ':')
pub fn parse_command(input: &str) -> Command {
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
                // No argument - show current batch window
                Command::ShowBatchWindow
            } else {
                // Check for presets first
                match parts[1] {
                    "fast" => Command::SetBatchWindow(100),
                    "medium" => Command::SetBatchWindow(1000),
                    "slow" => Command::SetBatchWindow(5000),
                    _ => {
                        // Try to parse as number
                        match parts[1].parse::<i64>() {
                            Ok(ms) if ms > 0 => Command::SetBatchWindow(ms),
                            Ok(_) => Command::Unknown("Batch window must be positive".to_string()),
                            Err(_) => Command::Unknown("Batch window must be a valid number or preset (fast/medium/slow)".to_string()),
                        }
                    }
                }
            }
        }
        "hide" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :hide <process> or :hide all".to_string())
            } else if parts[1] == "all" {
                Command::HideAll
            } else {
                Command::Hide(parts[1].to_string())
            }
        }
        "show" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :show <process> or :show all".to_string())
            } else if parts[1] == "all" {
                Command::ShowAll
            } else {
                Command::Show(parts[1].to_string())
            }
        }
        _ => Command::Unknown(format!("Unknown command: {}", parts[0])),
    }
}

/// Command executor that handles command execution
pub struct CommandExecutor<'a> {
    app: &'a mut App,
    manager: &'a mut ProcessManager,
    config: &'a mut Config,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(app: &'a mut App, manager: &'a mut ProcessManager, config: &'a mut Config) -> Self {
        Self { app, manager, config }
    }

    pub async fn execute(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Quit => {
                self.app.quit();
            }
            Command::Start(name) => {
                self.execute_start(&name).await?;
            }
            Command::Restart(name) => {
                self.execute_restart(&name).await?;
            }
            Command::Kill(name) => {
                self.execute_kill(&name).await?;
            }
            Command::FilterInclude(pattern) => {
                self.execute_filter_include(pattern);
            }
            Command::FilterExclude(pattern) => {
                self.execute_filter_exclude(pattern);
            }
            Command::FilterClear => {
                self.execute_filter_clear();
            }
            Command::FilterList => {
                self.execute_filter_list();
            }
            Command::NextBatch => {
                self.execute_next_batch();
            }
            Command::PrevBatch => {
                self.execute_prev_batch();
            }
            Command::ShowBatch => {
                self.execute_show_batch();
            }
            Command::SetBatchWindow(ms) => {
                self.execute_set_batch_window(ms);
            }
            Command::ShowBatchWindow => {
                self.execute_show_batch_window();
            }
            Command::Hide(process) => {
                self.execute_hide(process);
            }
            Command::Show(process) => {
                self.execute_show(process);
            }
            Command::HideAll => {
                self.execute_hide_all();
            }
            Command::ShowAll => {
                self.execute_show_all();
            }
            Command::Unknown(msg) => {
                self.app.set_status_error(format!("Error: {}", msg));
            }
        }
        Ok(())
    }

    async fn execute_start(&mut self, name: &str) -> Result<()> {
        match self.manager.start_process(name).await {
            Ok(_) => {
                self.app.set_status_success(format!("Started process: {}", name));
            }
            Err(e) => {
                self.app.set_status_error(format!("Failed to start {}: {}", name, e));
            }
        }
        Ok(())
    }

    async fn execute_restart(&mut self, name: &str) -> Result<()> {
        match self.manager.restart_process(name).await {
            Ok(_) => {
                self.app.set_status_success(format!("Restarted process: {}", name));
            }
            Err(e) => {
                self.app.set_status_error(format!("Failed to restart {}: {}", name, e));
            }
        }
        Ok(())
    }

    async fn execute_kill(&mut self, name: &str) -> Result<()> {
        match self.manager.kill_process(name).await {
            Ok(_) => {
                self.app.set_status_success(format!("Killed process: {}", name));
            }
            Err(e) => {
                self.app.set_status_error(format!("Failed to kill {}: {}", name, e));
            }
        }
        Ok(())
    }

    fn execute_filter_include(&mut self, pattern: String) {
        filter::add_include_filter(self.app, self.config, pattern.clone());
        self.app.set_status_success(format!("Added include filter: {}", pattern));
    }

    fn execute_filter_exclude(&mut self, pattern: String) {
        filter::add_exclude_filter(self.app, self.config, pattern.clone());
        self.app.set_status_success(format!("Added exclude filter: {}", pattern));
    }

    fn execute_filter_clear(&mut self) {
        let count = filter::clear_filters(self.app, self.config);
        self.app.set_status_success(format!("Cleared {} filter(s)", count));
    }

    fn execute_filter_list(&mut self) {
        match filter::list_filters(self.app) {
            Some(msg) => self.app.set_status_info(msg),
            None => self.app.set_status_info("No active filters".to_string()),
        }
    }

    fn execute_next_batch(&mut self) {
        batch::next_batch(self.app, self.manager);
        self.app.set_status_info("Next batch".to_string());
    }

    fn execute_prev_batch(&mut self) {
        batch::prev_batch(self.app, self.manager);
        self.app.set_status_info("Previous batch".to_string());
    }

    fn execute_show_batch(&mut self) {
        let enabled = batch::toggle_batch_view(self.app, self.manager);
        if enabled {
            self.app.set_status_info("Batch view mode enabled".to_string());
        } else {
            self.app.set_status_info("Batch view mode disabled".to_string());
        }
    }

    fn execute_set_batch_window(&mut self, ms: i64) {
        let batch_count = batch_window::set_batch_window(self.app, self.manager, self.config, ms);
        self.app.set_status_success(format!("Batch window set to {}ms ({} batches detected)", ms, batch_count));
    }

    fn execute_show_batch_window(&mut self) {
        self.app.set_status_info(format!("Current batch window: {}ms", self.app.batch_window_ms));
    }

    fn execute_hide(&mut self, process: String) {
        if self.manager.has_process(&process) {
            self.app.hidden_processes.insert(process.clone());
            self.app.set_status_success(format!("Hidden: {}", process));

            // Save to config
            self.sync_hidden_processes_to_config();
            self.save_config();
        } else {
            self.app.set_status_error(format!("Process not found: {}", process));
        }
    }

    fn execute_show(&mut self, process: String) {
        if self.app.hidden_processes.remove(&process) {
            self.app.set_status_success(format!("Shown: {}", process));

            // Save to config
            self.sync_hidden_processes_to_config();
            self.save_config();
        } else {
            self.app.set_status_info(format!("Process was not hidden: {}", process));
        }
    }

    fn execute_hide_all(&mut self) {
        let all_processes: Vec<String> = self.manager
            .get_processes()
            .keys()
            .cloned()
            .collect();

        for process in all_processes {
            self.app.hidden_processes.insert(process);
        }

        self.app.set_status_success("Hidden all processes".to_string());

        // Save to config
        self.sync_hidden_processes_to_config();
        self.save_config();
    }

    fn execute_show_all(&mut self) {
        let count = self.app.hidden_processes.len();
        self.app.hidden_processes.clear();

        self.app.set_status_success(format!("Shown all processes ({} were hidden)", count));

        // Save to config
        self.sync_hidden_processes_to_config();
        self.save_config();
    }

    fn sync_hidden_processes_to_config(&mut self) {
        self.config.hidden_processes = self.app.hidden_processes.iter().cloned().collect();
    }

    fn save_config(&mut self) {
        self.config.update_filters(&self.app.filters);
        if let Some(path) = &self.config.config_path {
            if let Err(e) = self.config.save(path.to_str().unwrap()) {
                self.app.set_status_error(format!("Config save failed: {}", e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bw_command_valid_values() {
        // Test valid batch window values
        match parse_command("bw 1000") {
            Command::SetBatchWindow(1000) => {},
            _ => panic!("Expected SetBatchWindow(1000)"),
        }

        match parse_command("bw 50") {
            Command::SetBatchWindow(50) => {},
            _ => panic!("Expected SetBatchWindow(50)"),
        }

        match parse_command("bw 5000") {
            Command::SetBatchWindow(5000) => {},
            _ => panic!("Expected SetBatchWindow(5000)"),
        }

        match parse_command("bw 1") {
            Command::SetBatchWindow(1) => {},
            _ => panic!("Expected SetBatchWindow(1)"),
        }
    }

    #[test]
    fn test_parse_bw_command_negative_value() {
        // Test that negative values are rejected
        match parse_command("bw -100") {
            Command::Unknown(msg) => {
                assert!(msg.contains("positive"), "Expected error about positive value, got: {}", msg);
            },
            _ => panic!("Expected Unknown command for negative value"),
        }
    }

    #[test]
    fn test_parse_bw_command_zero_value() {
        // Test that zero is rejected
        match parse_command("bw 0") {
            Command::Unknown(msg) => {
                assert!(msg.contains("positive"), "Expected error about positive value, got: {}", msg);
            },
            _ => panic!("Expected Unknown command for zero value"),
        }
    }

    #[test]
    fn test_parse_bw_command_non_numeric() {
        // Test that invalid non-numeric values are rejected
        match parse_command("bw abc") {
            Command::Unknown(msg) => {
                assert!(msg.contains("valid number") || msg.contains("preset"), "Expected error about valid number or preset, got: {}", msg);
            },
            _ => panic!("Expected Unknown command for non-numeric value"),
        }

        match parse_command("bw invalid") {
            Command::Unknown(msg) => {
                assert!(msg.contains("valid number") || msg.contains("preset"), "Expected error about valid number or preset, got: {}", msg);
            },
            _ => panic!("Expected Unknown command for invalid preset"),
        }
    }

    #[test]
    fn test_parse_bw_command_missing_argument() {
        // Test that missing argument returns ShowBatchWindow
        match parse_command("bw") {
            Command::ShowBatchWindow => {},
            _ => panic!("Expected ShowBatchWindow for missing argument"),
        }
    }

    #[test]
    fn test_parse_bw_command_extra_whitespace() {
        // Test that extra whitespace doesn't break parsing
        match parse_command("bw  1000") {
            Command::SetBatchWindow(1000) => {},
            _ => panic!("Expected SetBatchWindow(1000) with extra whitespace"),
        }

        match parse_command("  bw 500  ") {
            Command::SetBatchWindow(500) => {},
            _ => panic!("Expected SetBatchWindow(500) with surrounding whitespace"),
        }
    }

    #[test]
    fn test_parse_bw_command_presets() {
        // Test preset values: fast, medium, slow
        match parse_command("bw fast") {
            Command::SetBatchWindow(100) => {},
            _ => panic!("Expected SetBatchWindow(100) for 'fast' preset"),
        }

        match parse_command("bw medium") {
            Command::SetBatchWindow(1000) => {},
            _ => panic!("Expected SetBatchWindow(1000) for 'medium' preset"),
        }

        match parse_command("bw slow") {
            Command::SetBatchWindow(5000) => {},
            _ => panic!("Expected SetBatchWindow(5000) for 'slow' preset"),
        }
    }

    #[test]
    fn test_parse_bw_command_show_current() {
        // Test showing current batch window value
        match parse_command("bw") {
            Command::ShowBatchWindow => {},
            _ => panic!("Expected ShowBatchWindow"),
        }
    }
}
