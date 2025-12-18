use crate::config::StatusConfig;
use ratatui::style::Color;
use regex::Regex;

pub struct CompiledTransition {
    pub regex: Regex,
    pub label: String,
    pub color: Option<Color>,
}

pub struct StatusMatcher {
    default: Option<String>,
    transitions: Vec<CompiledTransition>,
    current_label: Option<String>,
    current_color: Option<Color>,
}

fn parse_color(s: &str) -> Option<Color> {
    match s.to_lowercase().as_str() {
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "red" => Some(Color::Red),
        "blue" => Some(Color::Blue),
        "cyan" => Some(Color::Cyan),
        "magenta" => Some(Color::Magenta),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        _ => None,
    }
}

impl StatusMatcher {
    /// Create from config. Returns Err if any pattern fails to compile.
    pub fn new(config: &StatusConfig) -> Result<Self, regex::Error> {
        let mut transitions = Vec::with_capacity(config.transitions.len());

        for t in &config.transitions {
            let regex = Regex::new(&t.pattern)?;
            let color = t.color.as_ref().and_then(|c| parse_color(c));
            transitions.push(CompiledTransition {
                regex,
                label: t.label.clone(),
                color,
            });
        }

        Ok(StatusMatcher {
            default: config.default.clone(),
            transitions,
            current_label: config.default.clone(),
            current_color: None,
        })
    }

    /// Check log line against patterns. Returns true if status changed.
    pub fn check_line(&mut self, line: &str) -> bool {
        for t in &self.transitions {
            if t.regex.is_match(line) {
                let changed = self.current_label.as_ref() != Some(&t.label);
                self.current_label = Some(t.label.clone());
                self.current_color = t.color;
                return changed;
            }
        }
        false
    }

    /// Get current display status. Returns None if no custom status.
    pub fn get_display_status(&self) -> Option<(&str, Option<Color>)> {
        self.current_label
            .as_ref()
            .map(|l| (l.as_str(), self.current_color))
    }

    /// Reset to default (call when process restarts).
    pub fn reset(&mut self) {
        self.current_label = self.default.clone();
        self.current_color = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StatusTransition;

    fn make_config(
        default: Option<&str>,
        transitions: Vec<(&str, &str, Option<&str>)>,
    ) -> StatusConfig {
        StatusConfig {
            default: default.map(|s| s.to_string()),
            transitions: transitions
                .into_iter()
                .map(|(pattern, label, color)| StatusTransition {
                    pattern: pattern.to_string(),
                    label: label.to_string(),
                    color: color.map(|c| c.to_string()),
                })
                .collect(),
        }
    }

    #[test]
    fn test_new_with_valid_patterns() {
        let config = make_config(
            Some("Starting"),
            vec![
                ("compiled successfully", "Ready", Some("green")),
                ("Compiling|Rebuilding", "Building", Some("yellow")),
            ],
        );

        let matcher = StatusMatcher::new(&config).unwrap();
        assert_eq!(matcher.transitions.len(), 2);
    }

    #[test]
    fn test_new_with_invalid_pattern() {
        let config = make_config(None, vec![("(unclosed", "Bad", None)]);

        let result = StatusMatcher::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_label_on_creation() {
        let config = make_config(Some("Preparing"), vec![]);

        let matcher = StatusMatcher::new(&config).unwrap();

        let status = matcher.get_display_status();
        assert!(status.is_some());
        let (label, color) = status.unwrap();
        assert_eq!(label, "Preparing");
        assert!(color.is_none());
    }

    #[test]
    fn test_no_default_returns_none() {
        let config = make_config(None, vec![("Ready", "Ready", None)]);

        let matcher = StatusMatcher::new(&config).unwrap();

        let status = matcher.get_display_status();
        assert!(status.is_none());
    }

    #[test]
    fn test_transition_changes_label() {
        let config = make_config(
            Some("Starting"),
            vec![("webpack compiled", "Ready", Some("green"))],
        );

        let mut matcher = StatusMatcher::new(&config).unwrap();

        let changed = matcher.check_line("webpack compiled successfully");
        assert!(changed);

        let status = matcher.get_display_status().unwrap();
        assert_eq!(status.0, "Ready");
        assert_eq!(status.1, Some(Color::Green));
    }

    #[test]
    fn test_first_match_wins() {
        let config = make_config(
            None,
            vec![
                ("error", "Error", Some("red")),
                ("error|warning", "Problem", Some("yellow")),
            ],
        );

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("error occurred");

        let status = matcher.get_display_status().unwrap();
        assert_eq!(status.0, "Error");
        assert_eq!(status.1, Some(Color::Red));
    }

    #[test]
    fn test_no_match_no_change() {
        let config = make_config(Some("Starting"), vec![("Ready", "Ready", Some("green"))]);

        let mut matcher = StatusMatcher::new(&config).unwrap();

        let changed = matcher.check_line("Some unrelated log line");
        assert!(!changed);

        let status = matcher.get_display_status().unwrap();
        assert_eq!(status.0, "Starting");
    }

    #[test]
    fn test_same_status_no_change() {
        let config = make_config(None, vec![("Ready", "Ready", Some("green"))]);

        let mut matcher = StatusMatcher::new(&config).unwrap();

        let first_change = matcher.check_line("Ready");
        assert!(first_change);

        let second_change = matcher.check_line("Ready again");
        assert!(!second_change);
    }

    #[test]
    fn test_reset_restores_default() {
        let config = make_config(
            Some("Starting"),
            vec![("Ready", "Ready", Some("green"))],
        );

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("Ready");
        assert_eq!(matcher.get_display_status().unwrap().0, "Ready");

        matcher.reset();
        assert_eq!(matcher.get_display_status().unwrap().0, "Starting");
        assert!(matcher.get_display_status().unwrap().1.is_none());
    }

    #[test]
    fn test_reset_with_no_default() {
        let config = make_config(None, vec![("Ready", "Ready", Some("green"))]);

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("Ready");
        assert!(matcher.get_display_status().is_some());

        matcher.reset();
        assert!(matcher.get_display_status().is_none());
    }

    #[test]
    fn test_color_parsing() {
        let config = make_config(
            None,
            vec![
                ("green", "Green", Some("green")),
                ("yellow", "Yellow", Some("YELLOW")),
                ("red", "Red", Some("Red")),
                ("blue", "Blue", Some("blue")),
                ("cyan", "Cyan", Some("cyan")),
                ("magenta", "Magenta", Some("magenta")),
                ("white", "White", Some("white")),
                ("gray", "Gray", Some("gray")),
                ("grey", "Grey", Some("grey")),
            ],
        );

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("green");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Green));

        matcher.check_line("yellow");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Yellow));

        matcher.check_line("red");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Red));

        matcher.check_line("blue");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Blue));

        matcher.check_line("cyan");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Cyan));

        matcher.check_line("magenta");
        assert_eq!(
            matcher.get_display_status().unwrap().1,
            Some(Color::Magenta)
        );

        matcher.check_line("white");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::White));

        matcher.check_line("gray");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Gray));

        matcher.check_line("grey");
        assert_eq!(matcher.get_display_status().unwrap().1, Some(Color::Gray));
    }

    #[test]
    fn test_unknown_color_is_none() {
        let config = make_config(None, vec![("test", "Test", Some("unknown_color"))]);

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("test");
        let status = matcher.get_display_status().unwrap();
        assert_eq!(status.0, "Test");
        assert!(status.1.is_none());
    }

    #[test]
    fn test_transition_without_color() {
        let config = make_config(None, vec![("Ready", "Ready", None)]);

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("Ready");
        let status = matcher.get_display_status().unwrap();
        assert_eq!(status.0, "Ready");
        assert!(status.1.is_none());
    }

    #[test]
    fn test_regex_pattern_matching() {
        let config = make_config(
            None,
            vec![("Compiling|Rebuilding|HMR", "Building", Some("yellow"))],
        );

        let mut matcher = StatusMatcher::new(&config).unwrap();

        matcher.check_line("Compiling...");
        assert_eq!(matcher.get_display_status().unwrap().0, "Building");

        matcher.reset();
        matcher.check_line("Rebuilding modules");
        assert_eq!(matcher.get_display_status().unwrap().0, "Building");

        matcher.reset();
        matcher.check_line("HMR update received");
        assert_eq!(matcher.get_display_status().unwrap().0, "Building");
    }
}
