use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProcfileConfig {
    File(PathBuf),
    Inline(HashMap<String, String>),
}

impl Default for ProcfileConfig {
    fn default() -> Self {
        Self::File(PathBuf::from("Procfile"))
    }
}

impl From<PathBuf> for ProcfileConfig {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl ProcfileConfig {
    pub fn load(&self) -> Result<Procfile> {
        match self {
            Self::File(path) => Procfile::from_file(path),
            Self::Inline(processes) => {
                anyhow::ensure!(
                    !processes.is_empty(),
                    "[procfile] contains no process definitions"
                );
                for (name, command) in processes {
                    anyhow::ensure!(!name.trim().is_empty(), "Empty process name in [procfile]");
                    anyhow::ensure!(
                        !command.trim().is_empty(),
                        "Empty command for process '{}' in [procfile]",
                        name
                    );
                }
                Ok(Procfile {
                    processes: processes.clone(),
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProcessSource {
    File(PathBuf),
    Config(PathBuf),
}

impl ProcessSource {
    pub fn resolve(config: &crate::config::Config, override_path: Option<&str>) -> Result<Self> {
        if let Some(path) = override_path {
            return Ok(Self::File(path.into()));
        }
        match &config.procfile {
            ProcfileConfig::File(path) => Ok(Self::File(path.clone())),
            ProcfileConfig::Inline(_) => Ok(Self::Config(
                config
                    .config_path
                    .clone()
                    .context("Inline processes require a config file path")?,
            )),
        }
    }

    pub fn load(&self) -> Result<Procfile> {
        match self {
            Self::File(path) => Procfile::from_file(path),
            Self::Config(path) => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config at {:?}", path))?;
                let config: crate::config::Config = toml::from_str(&content)?;
                anyhow::ensure!(
                    matches!(config.procfile, ProcfileConfig::Inline(_)),
                    "[procfile] was removed; restart oit to change process sources"
                );
                config.procfile.load()
            }
        }
    }

    pub fn working_dir(&self) -> Result<PathBuf> {
        let (Self::File(path) | Self::Config(path)) = self;
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
        Ok(std::env::current_dir()?.join(parent.unwrap_or(Path::new(""))))
    }
}

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
                    anyhow::bail!(
                        "Empty command for process '{}' on line {}",
                        name,
                        line_num + 1
                    );
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

    #[allow(dead_code)]
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
