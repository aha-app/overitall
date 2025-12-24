use crate::config::Config;
use crate::operations::{batch, batch_window, coloring, filter, goto, process, traces, visibility};
use crate::process::ProcessManager;
use crate::ui::App;
use anyhow::Result;

/// Target for goto command - absolute or relative time
#[derive(Debug, PartialEq, Clone)]
pub enum GotoTarget {
    AbsoluteTime { hour: u32, minute: u32, second: Option<u32> },
    RelativeTime { seconds: i64 },
}

/// Command parsed from user input
#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    Start(String),
    Restart(Option<String>),
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
    Only(String),
    Traces,
    ColorToggle,
    Goto(GotoTarget),
    Unknown(String),
}

/// Parse a command from user input (without the leading ':')
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Command::Unknown("Empty command".to_string());
    }

    match parts[0] {
        "q" | "quit" | "exit" => Command::Quit,
        "s" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :s <process>".to_string())
            } else {
                Command::Start(parts[1].to_string())
            }
        }
        "r" | "restart" => {
            if parts.len() < 2 {
                Command::Restart(None)
            } else {
                Command::Restart(Some(parts[1].to_string()))
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
        "only" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :only <process>".to_string())
            } else {
                Command::Only(parts[1].to_string())
            }
        }
        "traces" => Command::Traces,
        "color" | "colors" => Command::ColorToggle,
        "g" | "goto" => {
            if parts.len() < 2 {
                Command::Unknown("Usage: :goto HH:MM[:SS] or :goto +/-Ns/m/h".to_string())
            } else {
                parse_goto_target(parts[1])
            }
        }
        _ => Command::Unknown(format!("Unknown command: {}", parts[0])),
    }
}

/// Parse a goto target (absolute or relative time)
fn parse_goto_target(input: &str) -> Command {
    // Check for relative time format: +/-Ns, +/-Nm, +/-Nh
    let first_char = input.chars().next().unwrap_or(' ');
    if (first_char == '-' || first_char == '+') && input.len() >= 2 {
        let sign: i64 = if first_char == '-' { -1 } else { 1 };
        let last_char = input.chars().last().unwrap();
        if let Some(num_str) = input.get(1..input.len() - 1) {
            if let Ok(value) = num_str.parse::<i64>() {
                let seconds = match last_char {
                    's' => value,
                    'm' => value * 60,
                    'h' => value * 3600,
                    _ => return Command::Unknown(format!(
                        "Invalid time unit '{}'. Use s (seconds), m (minutes), or h (hours)",
                        last_char
                    )),
                };
                return Command::Goto(GotoTarget::RelativeTime { seconds: sign * seconds });
            }
        }
        return Command::Unknown("Invalid relative time format. Use +/-Ns, +/-Nm, or +/-Nh".to_string());
    }

    // Check for absolute time format: HH:MM or HH:MM:SS
    let parts: Vec<&str> = input.split(':').collect();
    match parts.len() {
        2 => {
            // HH:MM format
            match (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                (Ok(hour), Ok(minute)) if hour < 24 && minute < 60 => {
                    Command::Goto(GotoTarget::AbsoluteTime { hour, minute, second: None })
                }
                _ => Command::Unknown("Invalid time format. Use HH:MM (00:00 to 23:59)".to_string()),
            }
        }
        3 => {
            // HH:MM:SS format
            match (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                (Ok(hour), Ok(minute), Ok(second)) if hour < 24 && minute < 60 && second < 60 => {
                    Command::Goto(GotoTarget::AbsoluteTime { hour, minute, second: Some(second) })
                }
                _ => Command::Unknown("Invalid time format. Use HH:MM:SS (00:00:00 to 23:59:59)".to_string()),
            }
        }
        _ => Command::Unknown("Invalid format. Use HH:MM[:SS] or +/-Ns/m/h".to_string()),
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
            Command::Restart(Some(name)) => {
                self.execute_restart(&name)?;
            }
            Command::Restart(None) => {
                self.execute_restart_all()?;
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
            Command::Only(process) => {
                self.execute_only(process);
            }
            Command::Traces => {
                self.execute_traces();
            }
            Command::ColorToggle => {
                self.execute_color_toggle();
            }
            Command::Goto(target) => {
                self.execute_goto(target);
            }
            Command::Unknown(msg) => {
                self.app.display.set_status_error(format!("Error: {}", msg));
            }
        }
        Ok(())
    }

    async fn execute_start(&mut self, name: &str) -> Result<()> {
        // Check if it's a standalone log file first
        if self.manager.has_standalone_log_file(name) {
            self.app.display.set_status_error(format!("Cannot start log file: {}", name));
            return Ok(());
        }
        match process::start_process(self.manager, name).await {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
        Ok(())
    }

    fn execute_restart(&mut self, name: &str) -> Result<()> {
        // Check if it's a standalone log file first
        if self.manager.has_standalone_log_file(name) {
            self.app.display.set_status_error(format!("Cannot restart log file: {}", name));
            return Ok(());
        }
        // Non-blocking: just set the status and let the main loop handle the actual restart
        if self.manager.set_restarting(name) {
            self.app.display.set_status_info(format!("Restarting: {}", name));
        } else {
            self.app.display.set_status_error(format!("Process not found: {}", name));
        }
        Ok(())
    }

    fn execute_restart_all(&mut self) -> Result<()> {
        // Non-blocking: just set the status and let the main loop handle the actual restart
        let names: Vec<String> = self.manager.get_all_statuses().into_iter().map(|(n, _)| n).collect();
        if names.is_empty() {
            self.app.display.set_status_error("No processes to restart".to_string());
        } else {
            self.manager.set_all_restarting();
            self.app.display.set_status_info(format!("Restarting {} process(es)...", names.len()));
        }
        Ok(())
    }

    async fn execute_kill(&mut self, name: &str) -> Result<()> {
        // Check if it's a standalone log file first
        if self.manager.has_standalone_log_file(name) {
            self.app.display.set_status_error(format!("Cannot stop log file: {}", name));
            return Ok(());
        }
        match process::kill_process(self.manager, name).await {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
        }
        Ok(())
    }

    fn execute_filter_include(&mut self, pattern: String) {
        filter::add_include_filter(self.app, self.config, pattern.clone());
        self.app.display.set_status_success(format!("Added include filter: {}", pattern));
    }

    fn execute_filter_exclude(&mut self, pattern: String) {
        filter::add_exclude_filter(self.app, self.config, pattern.clone());
        self.app.display.set_status_success(format!("Added exclude filter: {}", pattern));
    }

    fn execute_filter_clear(&mut self) {
        let count = filter::clear_filters(self.app, self.config);
        self.app.display.set_status_success(format!("Cleared {} filter(s)", count));
    }

    fn execute_filter_list(&mut self) {
        match filter::list_filters(self.app) {
            Some(msg) => self.app.display.set_status_info(msg),
            None => self.app.display.set_status_info("No active filters".to_string()),
        }
    }

    fn execute_next_batch(&mut self) {
        batch::next_batch(self.app, self.manager);
        self.app.display.set_status_info("Next batch".to_string());
    }

    fn execute_prev_batch(&mut self) {
        batch::prev_batch(self.app, self.manager);
        self.app.display.set_status_info("Previous batch".to_string());
    }

    fn execute_show_batch(&mut self) {
        let enabled = batch::toggle_batch_view(self.app, self.manager);
        if enabled {
            self.app.display.set_status_info("Batch view mode enabled".to_string());
        } else {
            self.app.display.set_status_info("Batch view mode disabled".to_string());
        }
    }

    fn execute_set_batch_window(&mut self, ms: i64) {
        let batch_count = batch_window::set_batch_window(self.app, self.manager, self.config, ms);
        self.app.display.set_status_success(format!("Batch window set to {}ms ({} batches detected)", ms, batch_count));
    }

    fn execute_show_batch_window(&mut self) {
        self.app.display.set_status_info(format!("Current batch window: {}ms", self.app.batch.batch_window_ms));
    }

    fn execute_hide(&mut self, process: String) {
        match visibility::hide_process(self.app, self.manager, self.config, &process) {
            Ok(()) => self.app.display.set_status_success(format!("Hidden: {}", process)),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn execute_show(&mut self, process: String) {
        match visibility::show_process(self.app, self.config, &process) {
            Ok(true) => self.app.display.set_status_success(format!("Shown: {}", process)),
            Ok(false) => self.app.display.set_status_info(format!("Process was not hidden: {}", process)),
            Err(()) => {}
        }
    }

    fn execute_hide_all(&mut self) {
        visibility::hide_all(self.app, self.manager, self.config);
        self.app.display.set_status_success("Hidden all processes".to_string());
    }

    fn execute_show_all(&mut self) {
        let count = visibility::show_all(self.app, self.config);
        self.app.display.set_status_success(format!("Shown all processes ({} were hidden)", count));
    }

    fn execute_only(&mut self, process: String) {
        match visibility::only_process(self.app, self.manager, self.config, &process) {
            Ok(()) => self.app.display.set_status_success(format!("Showing only: {}", process)),
            Err(msg) => self.app.display.set_status_error(msg),
        }
    }

    fn execute_traces(&mut self) {
        traces::execute_traces(self.app, self.manager);
    }

    fn execute_color_toggle(&mut self) {
        let enabled = coloring::toggle_coloring(self.app, self.manager, self.config);
        if enabled {
            self.app.display.set_status_success("Process coloring enabled".to_string());
        } else {
            self.app.display.set_status_info("Process coloring disabled".to_string());
        }
    }

    fn execute_goto(&mut self, target: GotoTarget) {
        match goto::goto_timestamp(self.app, self.manager, target) {
            Ok(msg) => self.app.display.set_status_success(msg),
            Err(msg) => self.app.display.set_status_error(msg),
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

    #[test]
    fn test_parse_only_command() {
        match parse_command("only web") {
            Command::Only(name) => assert_eq!(name, "web"),
            _ => panic!("Expected Only(\"web\")"),
        }

        match parse_command("only worker") {
            Command::Only(name) => assert_eq!(name, "worker"),
            _ => panic!("Expected Only(\"worker\")"),
        }
    }

    #[test]
    fn test_parse_only_command_missing_argument() {
        match parse_command("only") {
            Command::Unknown(msg) => {
                assert!(msg.contains("Usage"), "Expected usage message, got: {}", msg);
            }
            _ => panic!("Expected Unknown command for missing argument"),
        }
    }

    #[test]
    fn test_parse_only_command_with_whitespace() {
        match parse_command("  only  web  ") {
            Command::Only(name) => assert_eq!(name, "web"),
            _ => panic!("Expected Only(\"web\") with surrounding whitespace"),
        }
    }

    #[test]
    fn test_parse_restart_with_process() {
        match parse_command("r web") {
            Command::Restart(Some(name)) => assert_eq!(name, "web"),
            _ => panic!("Expected Restart(Some(\"web\"))"),
        }

        match parse_command("restart worker") {
            Command::Restart(Some(name)) => assert_eq!(name, "worker"),
            _ => panic!("Expected Restart(Some(\"worker\"))"),
        }
    }

    #[test]
    fn test_parse_restart_without_process() {
        match parse_command("r") {
            Command::Restart(None) => {}
            _ => panic!("Expected Restart(None)"),
        }

        match parse_command("restart") {
            Command::Restart(None) => {}
            _ => panic!("Expected Restart(None)"),
        }
    }

    #[test]
    fn test_parse_restart_with_whitespace() {
        match parse_command("  r  web  ") {
            Command::Restart(Some(name)) => assert_eq!(name, "web"),
            _ => panic!("Expected Restart(Some(\"web\")) with whitespace"),
        }

        match parse_command("  restart  ") {
            Command::Restart(None) => {}
            _ => panic!("Expected Restart(None) with whitespace"),
        }
    }

    #[test]
    fn test_parse_quit_command() {
        assert_eq!(parse_command("q"), Command::Quit);
        assert_eq!(parse_command("quit"), Command::Quit);
        assert_eq!(parse_command("exit"), Command::Quit);
    }

    #[test]
    fn test_parse_quit_command_with_whitespace() {
        assert_eq!(parse_command("  q  "), Command::Quit);
        assert_eq!(parse_command("  quit  "), Command::Quit);
        assert_eq!(parse_command("  exit  "), Command::Quit);
    }

    #[test]
    fn test_parse_goto_absolute_time_hhmm() {
        match parse_command("goto 14:30") {
            Command::Goto(GotoTarget::AbsoluteTime { hour, minute, second }) => {
                assert_eq!(hour, 14);
                assert_eq!(minute, 30);
                assert_eq!(second, None);
            }
            other => panic!("Expected Goto AbsoluteTime, got {:?}", other),
        }

        match parse_command("g 09:05") {
            Command::Goto(GotoTarget::AbsoluteTime { hour, minute, second }) => {
                assert_eq!(hour, 9);
                assert_eq!(minute, 5);
                assert_eq!(second, None);
            }
            other => panic!("Expected Goto AbsoluteTime, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_absolute_time_hhmmss() {
        match parse_command("goto 14:30:45") {
            Command::Goto(GotoTarget::AbsoluteTime { hour, minute, second }) => {
                assert_eq!(hour, 14);
                assert_eq!(minute, 30);
                assert_eq!(second, Some(45));
            }
            other => panic!("Expected Goto AbsoluteTime with seconds, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_relative_seconds() {
        match parse_command("goto -30s") {
            Command::Goto(GotoTarget::RelativeTime { seconds }) => {
                assert_eq!(seconds, -30);
            }
            other => panic!("Expected Goto RelativeTime, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_relative_minutes() {
        match parse_command("g -5m") {
            Command::Goto(GotoTarget::RelativeTime { seconds }) => {
                assert_eq!(seconds, -300); // 5 * 60
            }
            other => panic!("Expected Goto RelativeTime, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_relative_hours() {
        match parse_command("goto -2h") {
            Command::Goto(GotoTarget::RelativeTime { seconds }) => {
                assert_eq!(seconds, -7200); // 2 * 3600
            }
            other => panic!("Expected Goto RelativeTime, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_missing_argument() {
        match parse_command("goto") {
            Command::Unknown(msg) => {
                assert!(msg.contains("Usage"), "Expected usage message, got: {}", msg);
            }
            other => panic!("Expected Unknown for missing argument, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_invalid_time() {
        match parse_command("goto 25:00") {
            Command::Unknown(msg) => {
                assert!(msg.contains("Invalid"), "Expected invalid message, got: {}", msg);
            }
            other => panic!("Expected Unknown for invalid hour, got {:?}", other),
        }

        match parse_command("goto 12:60") {
            Command::Unknown(msg) => {
                assert!(msg.contains("Invalid"), "Expected invalid message, got: {}", msg);
            }
            other => panic!("Expected Unknown for invalid minute, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_invalid_relative() {
        match parse_command("goto -5x") {
            Command::Unknown(msg) => {
                assert!(msg.contains("time unit"), "Expected time unit error, got: {}", msg);
            }
            other => panic!("Expected Unknown for invalid unit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_positive_relative_seconds() {
        match parse_command("goto +30s") {
            Command::Goto(GotoTarget::RelativeTime { seconds }) => {
                assert_eq!(seconds, 30);
            }
            other => panic!("Expected Goto RelativeTime positive, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_goto_positive_relative_minutes() {
        match parse_command("g +5m") {
            Command::Goto(GotoTarget::RelativeTime { seconds }) => {
                assert_eq!(seconds, 300); // 5 * 60
            }
            other => panic!("Expected Goto RelativeTime positive, got {:?}", other),
        }
    }
}
