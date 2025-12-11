use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents a parsed Procfile containing process definitions
#[derive(Debug, Clone)]
pub struct Procfile {
    /// Map of process names to their commands
    pub processes: HashMap<String, String>,
}

impl Procfile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Procfile at {:?}", path.as_ref()))?;

        Self::from_string(&content)
    }

    pub fn from_string(content: &str) -> Result<Self> {
        let mut processes = HashMap::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse the line as "name: command"
            if let Some((name, command)) = line.split_once(':') {
                let name = name.trim().to_string();
                let command = command.trim().to_string();

                if name.is_empty() {
                    anyhow::bail!("Empty process name on line {}", line_num + 1);
                }

                if command.is_empty() {
                    anyhow::bail!("Empty command for process '{}' on line {}", name, line_num + 1);
                }

                if processes.contains_key(&name) {
                    anyhow::bail!("Duplicate process name '{}' on line {}", name, line_num + 1);
                }

                processes.insert(name, command);
            } else {
                anyhow::bail!(
                    "Invalid Procfile syntax on line {}: expected 'name: command'",
                    line_num + 1
                );
            }
        }

        if processes.is_empty() {
            anyhow::bail!("Procfile contains no process definitions");
        }

        Ok(Procfile { processes })
    }

    pub fn process_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.processes.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn get_command(&self, name: &str) -> Option<&str> {
        self.processes.get(name).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_procfile() {
        let content = r#"
web: bundle exec rails server -p 3000
worker: bundle exec sidekiq
"#;
        let procfile = Procfile::from_string(content).unwrap();
        assert_eq!(procfile.processes.len(), 2);
        assert_eq!(
            procfile.get_command("web"),
            Some("bundle exec rails server -p 3000")
        );
        assert_eq!(procfile.get_command("worker"), Some("bundle exec sidekiq"));
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r#"
# This is a comment
web: rails server

# Another comment
worker: sidekiq
"#;
        let procfile = Procfile::from_string(content).unwrap();
        assert_eq!(procfile.processes.len(), 2);
    }

    #[test]
    fn test_empty_procfile_fails() {
        let content = "# Only comments\n\n";
        assert!(Procfile::from_string(content).is_err());
    }

    #[test]
    fn test_duplicate_process_fails() {
        let content = r#"
web: rails server
web: another command
"#;
        assert!(Procfile::from_string(content).is_err());
    }

    #[test]
    fn test_invalid_syntax_fails() {
        let content = "web rails server";
        assert!(Procfile::from_string(content).is_err());
    }

    #[test]
    fn test_process_names_sorted() {
        let content = r#"
zebra: command 1
alpha: command 2
middle: command 3
"#;
        let procfile = Procfile::from_string(content).unwrap();
        assert_eq!(procfile.process_names(), vec!["alpha", "middle", "zebra"]);
    }
}
