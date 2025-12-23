use ratatui::style::Color;
use std::collections::HashMap;

/// Default palette of distinct colors for process names.
/// Chosen for visibility on dark terminals, avoiding red (error-associated).
const DEFAULT_PALETTE: &[Color] = &[
    Color::Green,
    Color::Yellow,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightCyan,
    Color::LightBlue,
    Color::LightMagenta,
];

pub struct ProcessColors {
    assignments: HashMap<String, Color>,
}

impl ProcessColors {
    /// Create with config overrides and process/log file names.
    pub fn new(
        process_names: &[String],
        log_file_names: &[String],
        config_colors: &HashMap<String, String>,
    ) -> Self {
        let mut assignments = HashMap::new();

        // Combine all names in sorted order for deterministic assignment
        let mut all_names: Vec<&String> = process_names.iter().chain(log_file_names.iter()).collect();
        all_names.sort();
        all_names.dedup();

        for (idx, name) in all_names.iter().enumerate() {
            let color = if let Some(color_name) = config_colors.get(*name) {
                parse_color_name(color_name).unwrap_or(DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()])
            } else {
                DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()]
            };
            assignments.insert((*name).clone(), color);
        }

        Self { assignments }
    }

    pub fn get(&self, name: &str) -> Color {
        self.assignments.get(name).copied().unwrap_or(Color::White)
    }

    /// Get ANSI escape sequence for the process color.
    /// Returns (start_code, reset_code) to wrap text with color.
    pub fn get_ansi(&self, name: &str) -> (&'static str, &'static str) {
        let color = self.get(name);
        color_to_ansi(color)
    }
}

/// Convert ratatui Color to ANSI escape sequences.
fn color_to_ansi(color: Color) -> (&'static str, &'static str) {
    let reset = "\x1b[0m";
    let code = match color {
        Color::Red => "\x1b[31m",
        Color::Green => "\x1b[32m",
        Color::Yellow => "\x1b[33m",
        Color::Blue => "\x1b[34m",
        Color::Magenta => "\x1b[35m",
        Color::Cyan => "\x1b[36m",
        Color::White => "\x1b[37m",
        Color::Gray => "\x1b[90m",
        Color::DarkGray => "\x1b[90m",
        Color::LightRed => "\x1b[91m",
        Color::LightGreen => "\x1b[92m",
        Color::LightYellow => "\x1b[93m",
        Color::LightBlue => "\x1b[94m",
        Color::LightMagenta => "\x1b[95m",
        Color::LightCyan => "\x1b[96m",
        _ => "", // No color for unsupported types
    };
    (code, reset)
}

fn parse_color_name(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "darkgray" => Some(Color::DarkGray),
        "light_red" | "lightred" | "bright_red" => Some(Color::LightRed),
        "light_green" | "lightgreen" | "bright_green" => Some(Color::LightGreen),
        "light_yellow" | "lightyellow" | "bright_yellow" => Some(Color::LightYellow),
        "light_blue" | "lightblue" | "bright_blue" => Some(Color::LightBlue),
        "light_magenta" | "lightmagenta" | "bright_magenta" => Some(Color::LightMagenta),
        "light_cyan" | "lightcyan" | "bright_cyan" => Some(Color::LightCyan),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_palette_assignment() {
        let process_names = vec!["api".to_string(), "web".to_string(), "worker".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        // Sorted order: api, web, worker
        assert_eq!(colors.get("api"), Color::Green);
        assert_eq!(colors.get("web"), Color::Yellow);
        assert_eq!(colors.get("worker"), Color::Cyan);
    }

    #[test]
    fn test_config_override() {
        let process_names = vec!["api".to_string(), "web".to_string()];
        let log_file_names = vec![];
        let mut config_colors = HashMap::new();
        config_colors.insert("api".to_string(), "magenta".to_string());

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        assert_eq!(colors.get("api"), Color::Magenta);
        assert_eq!(colors.get("web"), Color::Yellow); // default palette, second position
    }

    #[test]
    fn test_color_name_parsing() {
        assert_eq!(parse_color_name("red"), Some(Color::Red));
        assert_eq!(parse_color_name("GREEN"), Some(Color::Green));
        assert_eq!(parse_color_name("Yellow"), Some(Color::Yellow));
        assert_eq!(parse_color_name("blue"), Some(Color::Blue));
        assert_eq!(parse_color_name("magenta"), Some(Color::Magenta));
        assert_eq!(parse_color_name("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color_name("white"), Some(Color::White));
        assert_eq!(parse_color_name("gray"), Some(Color::Gray));
        assert_eq!(parse_color_name("grey"), Some(Color::Gray));
        assert_eq!(parse_color_name("dark_gray"), Some(Color::DarkGray));
        assert_eq!(parse_color_name("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color_name("light_red"), Some(Color::LightRed));
        assert_eq!(parse_color_name("lightred"), Some(Color::LightRed));
        assert_eq!(parse_color_name("bright_red"), Some(Color::LightRed));
        assert_eq!(parse_color_name("invalid"), None);
    }

    #[test]
    fn test_palette_cycling() {
        // Create more processes than palette colors to test cycling
        let process_names: Vec<String> = (0..12).map(|i| format!("proc{:02}", i)).collect();
        let log_file_names = vec![];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        // After 10 colors, should cycle back to Green
        assert_eq!(colors.get("proc00"), Color::Green);
        assert_eq!(colors.get("proc10"), Color::Green); // 11th process, wraps to first color
    }

    #[test]
    fn test_unknown_process_returns_white() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        assert_eq!(colors.get("unknown"), Color::White);
    }

    #[test]
    fn test_log_files_included() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec!["rails".to_string()];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        // Sorted order: api, rails
        assert_eq!(colors.get("api"), Color::Green);
        assert_eq!(colors.get("rails"), Color::Yellow);
    }

    #[test]
    fn test_deterministic_assignment() {
        // Same inputs should produce same color assignments
        let process_names = vec!["web".to_string(), "api".to_string()];
        let log_file_names = vec!["rails".to_string()];
        let config_colors = HashMap::new();

        let colors1 = ProcessColors::new(&process_names, &log_file_names, &config_colors);
        let colors2 = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        assert_eq!(colors1.get("api"), colors2.get("api"));
        assert_eq!(colors1.get("web"), colors2.get("web"));
        assert_eq!(colors1.get("rails"), colors2.get("rails"));
    }

    #[test]
    fn test_invalid_config_color_uses_default() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec![];
        let mut config_colors = HashMap::new();
        config_colors.insert("api".to_string(), "invalidcolor".to_string());

        let colors = ProcessColors::new(&process_names, &log_file_names, &config_colors);

        // Falls back to first palette color since "invalidcolor" is invalid
        assert_eq!(colors.get("api"), Color::Green);
    }
}
