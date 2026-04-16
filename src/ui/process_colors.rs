use ratatui::style::Color;
use std::collections::HashMap;

const ANSI_RESET: &str = "\x1b[0m";

struct Assignment {
    color: Color,
    ansi_start: String,
}

pub struct ProcessColors {
    assignments: HashMap<String, Assignment>,
    fallback: Color,
}

impl ProcessColors {
    /// Create with config overrides, process/log file names, a palette, and a fallback color.
    /// The palette comes from the active `Theme` (see `ui::theme`).
    pub fn new(
        process_names: &[String],
        log_file_names: &[String],
        config_colors: &HashMap<String, String>,
        palette: &[Color],
        fallback: Color,
    ) -> Self {
        let mut assignments = HashMap::new();

        // Combine all names in sorted order for deterministic assignment
        let mut all_names: Vec<&String> = process_names.iter().chain(log_file_names.iter()).collect();
        all_names.sort();
        all_names.dedup();

        let palette_len = palette.len().max(1);
        for (idx, name) in all_names.iter().enumerate() {
            let default_color = palette
                .get(idx % palette_len)
                .copied()
                .unwrap_or(fallback);
            let color = if let Some(color_name) = config_colors.get(*name) {
                parse_color_name(color_name).unwrap_or(default_color)
            } else {
                default_color
            };
            assignments.insert(
                (*name).clone(),
                Assignment {
                    color,
                    ansi_start: color_to_ansi_start(color),
                },
            );
        }

        Self {
            assignments,
            fallback,
        }
    }

    pub fn get(&self, name: &str) -> Color {
        self.assignments
            .get(name)
            .map(|a| a.color)
            .unwrap_or(self.fallback)
    }

    /// Get ANSI escape sequence for the process color.
    /// Returns (start_code, reset_code) to wrap text with color.
    pub fn get_ansi(&self, name: &str) -> (&str, &'static str) {
        match self.assignments.get(name) {
            Some(a) => (a.ansi_start.as_str(), ANSI_RESET),
            None => ("", ANSI_RESET),
        }
    }
}

/// Convert ratatui Color to an ANSI start escape sequence.
fn color_to_ansi_start(color: Color) -> String {
    match color {
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::White => "\x1b[37m".to_string(),
        Color::Gray => "\x1b[90m".to_string(),
        Color::DarkGray => "\x1b[90m".to_string(),
        Color::LightRed => "\x1b[91m".to_string(),
        Color::LightGreen => "\x1b[92m".to_string(),
        Color::LightYellow => "\x1b[93m".to_string(),
        Color::LightBlue => "\x1b[94m".to_string(),
        Color::LightMagenta => "\x1b[95m".to_string(),
        Color::LightCyan => "\x1b[96m".to_string(),
        Color::Black => "\x1b[30m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        Color::Indexed(i) => format!("\x1b[38;5;{}m", i),
        _ => String::new(),
    }
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
    use crate::ui::theme::Theme;

    fn dark_palette() -> &'static [Color] {
        Theme::dark().process_palette
    }

    fn dark_fallback() -> Color {
        Theme::dark().fallback_process
    }

    #[test]
    fn test_default_palette_assignment() {
        let process_names = vec!["api".to_string(), "web".to_string(), "worker".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

        // Sorted order: api, web, worker
        assert_eq!(colors.get("api"), Color::Green);
        assert_eq!(colors.get("web"), Color::Yellow);
        assert_eq!(colors.get("worker"), Color::Cyan);
    }

    #[test]
    fn test_light_palette_assignment() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();
        let light = Theme::light();

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            light.process_palette,
            light.fallback_process,
        );

        assert_eq!(colors.get("api"), Color::Blue);
    }

    #[test]
    fn test_config_override() {
        let process_names = vec!["api".to_string(), "web".to_string()];
        let log_file_names = vec![];
        let mut config_colors = HashMap::new();
        config_colors.insert("api".to_string(), "magenta".to_string());

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

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

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

        // After 10 colors, should cycle back to Green
        assert_eq!(colors.get("proc00"), Color::Green);
        assert_eq!(colors.get("proc10"), Color::Green); // 11th process, wraps to first color
    }

    #[test]
    fn test_unknown_process_returns_fallback() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

        assert_eq!(colors.get("unknown"), Color::White);
    }

    #[test]
    fn test_log_files_included() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec!["rails".to_string()];
        let config_colors = HashMap::new();

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

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

        let colors1 = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );
        let colors2 = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

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

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            dark_palette(),
            dark_fallback(),
        );

        // Falls back to first palette color since "invalidcolor" is invalid
        assert_eq!(colors.get("api"), Color::Green);
    }

    #[test]
    fn test_rgb_color_emits_truecolor_ansi() {
        let process_names = vec!["api".to_string()];
        let log_file_names = vec![];
        let config_colors = HashMap::new();
        let light = Theme::light();

        let colors = ProcessColors::new(
            &process_names,
            &log_file_names,
            &config_colors,
            light.process_palette,
            light.fallback_process,
        );

        let (start, _reset) = colors.get_ansi("api");
        // Light palette[0] is Color::Blue → standard ANSI 34
        assert_eq!(start, "\x1b[34m");
    }
}
