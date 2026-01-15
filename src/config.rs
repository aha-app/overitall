use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub procfile: PathBuf,
    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,
    #[serde(default)]
    pub log_files: Vec<LogFileConfig>,
    #[serde(default)]
    pub filters: FilterConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_log_buffer_mb: Option<usize>,
    #[serde(default)]
    pub hidden_processes: Vec<String>,
    #[serde(default)]
    pub ignored_processes: Vec<String>,
    #[serde(default)]
    pub start_processes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_mode: Option<bool>,
    #[serde(default)]
    pub colors: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_coloring: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_copy_seconds: Option<f64>,
    #[serde(default)]
    pub groups: HashMap<String, Vec<String>>,

    // This field is not serialized, just used at runtime
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileConfig {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub transitions: Vec<StatusTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    pub pattern: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn update_filters(&mut self, app_filters: &[crate::ui::Filter]) {
        let mut include_filters = Vec::new();
        let mut exclude_filters = Vec::new();

        for filter in app_filters {
            match filter.filter_type {
                crate::ui::FilterType::Include => include_filters.push(filter.pattern.clone()),
                crate::ui::FilterType::Exclude => exclude_filters.push(filter.pattern.clone()),
            }
        }

        self.filters.include = include_filters;
        self.filters.exclude = exclude_filters;
    }

    pub fn validate(&self, process_names: &[String]) -> anyhow::Result<()> {
        use std::collections::HashSet;

        let process_set: HashSet<&str> = process_names.iter().map(|s| s.as_str()).collect();

        for log_file in &self.log_files {
            if process_set.contains(log_file.name.as_str()) {
                anyhow::bail!(
                    "Log file name '{}' conflicts with a process name",
                    log_file.name
                );
            }
        }

        let mut log_file_names: HashSet<&str> = HashSet::new();
        for log_file in &self.log_files {
            if !log_file_names.insert(&log_file.name) {
                anyhow::bail!("Duplicate log file name '{}'", log_file.name);
            }
        }

        for name in &self.start_processes {
            if !process_set.contains(name.as_str()) {
                anyhow::bail!("start_processes contains unknown process '{}'", name);
            }
        }

        for (group_name, members) in &self.groups {
            if process_set.contains(group_name.as_str()) {
                anyhow::bail!(
                    "Group name '{}' conflicts with a process name",
                    group_name
                );
            }
            if log_file_names.contains(group_name.as_str()) {
                anyhow::bail!(
                    "Group name '{}' conflicts with a log file name",
                    group_name
                );
            }
            if members.is_empty() {
                anyhow::bail!("Group '{}' cannot be empty", group_name);
            }
            for member in members {
                if !process_set.contains(member.as_str()) {
                    anyhow::bail!(
                        "Group '{}' contains unknown process '{}'",
                        group_name,
                        member
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_config() -> Config {
        Config {
            procfile: PathBuf::from("Procfile"),
            processes: HashMap::new(),
            log_files: Vec::new(),
            filters: FilterConfig::default(),
            batch_window_ms: None,
            max_log_buffer_mb: None,
            hidden_processes: Vec::new(),
            ignored_processes: Vec::new(),
            start_processes: Vec::new(),
            disable_auto_update: None,
            compact_mode: None,
            colors: HashMap::new(),
            process_coloring: None,
            context_copy_seconds: None,
            groups: HashMap::new(),
            config_path: None,
        }
    }

    #[test]
    fn test_batch_window_loads_from_config() {
        // Create a temp config file with batch_window_ms
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
batch_window_ms = 2000

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.batch_window_ms, Some(2000));
    }

    #[test]
    fn test_batch_window_defaults_when_missing() {
        // Create a temp config file WITHOUT batch_window_ms
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.batch_window_ms, None);
    }

    #[test]
    fn test_batch_window_saves_to_config() {
        let config = Config {
            batch_window_ms: Some(5000),
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.batch_window_ms, Some(5000));
    }

    #[test]
    fn test_batch_window_updates_in_config() {
        let mut config = test_config();

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        config.batch_window_ms = Some(3000);
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.batch_window_ms, Some(3000));
    }

    #[test]
    fn test_batch_window_none_not_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();

        assert!(
            !toml_string.contains("batch_window_ms"),
            "batch_window_ms should not be serialized when None"
        );
    }

    #[test]
    fn test_batch_window_some_is_serialized() {
        let config = Config {
            batch_window_ms: Some(1500),
            ..test_config()
        };

        let toml_string = toml::to_string_pretty(&config).unwrap();

        assert!(
            toml_string.contains("batch_window_ms = 1500"),
            "batch_window_ms should be serialized when Some(1500)"
        );
    }

    #[test]
    fn test_max_log_buffer_mb_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
max_log_buffer_mb = 100

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.max_log_buffer_mb, Some(100));
    }

    #[test]
    fn test_max_log_buffer_mb_defaults_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.max_log_buffer_mb, None);
    }

    #[test]
    fn test_max_log_buffer_mb_saves_to_config() {
        let config = Config {
            max_log_buffer_mb: Some(75),
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.max_log_buffer_mb, Some(75));
    }

    #[test]
    fn test_disable_auto_update_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
disable_auto_update = true

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.disable_auto_update, Some(true));
    }

    #[test]
    fn test_disable_auto_update_defaults_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.disable_auto_update, None);
    }

    #[test]
    fn test_disable_auto_update_none_not_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_string.contains("disable_auto_update"),
            "disable_auto_update should not be serialized when None"
        );
    }

    #[test]
    fn test_disable_auto_update_some_is_serialized() {
        let config = Config {
            disable_auto_update: Some(true),
            ..test_config()
        };

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("disable_auto_update = true"),
            "disable_auto_update should be serialized when Some(true)"
        );
    }

    #[test]
    fn test_hidden_processes_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
hidden_processes = ["web", "worker"]

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.hidden_processes, vec!["web", "worker"]);
    }

    #[test]
    fn test_hidden_processes_defaults_to_empty_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.hidden_processes.is_empty());
    }

    #[test]
    fn test_hidden_processes_saves_to_config() {
        let config = Config {
            hidden_processes: vec!["api".to_string(), "db".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.hidden_processes, vec!["api", "db"]);
    }

    #[test]
    fn test_hidden_processes_empty_array_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("hidden_processes = []"),
            "empty hidden_processes should be serialized as empty array"
        );
    }

    #[test]
    fn test_hidden_processes_roundtrip() {
        let original = Config {
            hidden_processes: vec!["web".to_string(), "worker".to_string(), "scheduler".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.hidden_processes, original.hidden_processes);
    }

    #[test]
    fn test_ignored_processes_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
ignored_processes = ["webpack", "worker"]

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.ignored_processes, vec!["webpack", "worker"]);
    }

    #[test]
    fn test_ignored_processes_defaults_to_empty_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.ignored_processes.is_empty());
    }

    #[test]
    fn test_ignored_processes_saves_to_config() {
        let config = Config {
            ignored_processes: vec!["webpack".to_string(), "scheduler".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.ignored_processes, vec!["webpack", "scheduler"]);
    }

    #[test]
    fn test_ignored_processes_empty_array_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("ignored_processes = []"),
            "empty ignored_processes should be serialized as empty array"
        );
    }

    #[test]
    fn test_ignored_processes_roundtrip() {
        let original = Config {
            ignored_processes: vec!["webpack".to_string(), "worker".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.ignored_processes, original.ignored_processes);
    }

    #[test]
    fn test_status_config_loads_from_toml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes.webpack]
log_file = "logs/webpack.log"

[processes.webpack.status]
default = "Preparing"
transitions = [
  {{ pattern = "webpack compiled", label = "Ready", color = "green" }},
  {{ pattern = "Rebuilding|Compiling", label = "Building", color = "yellow" }},
]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();

        let webpack_config = config.processes.get("webpack").unwrap();
        assert!(webpack_config.status.is_some());

        let status = webpack_config.status.as_ref().unwrap();
        assert_eq!(status.default, Some("Preparing".to_string()));
        assert_eq!(status.transitions.len(), 2);

        assert_eq!(status.transitions[0].pattern, "webpack compiled");
        assert_eq!(status.transitions[0].label, "Ready");
        assert_eq!(status.transitions[0].color, Some("green".to_string()));

        assert_eq!(status.transitions[1].pattern, "Rebuilding|Compiling");
        assert_eq!(status.transitions[1].label, "Building");
        assert_eq!(status.transitions[1].color, Some("yellow".to_string()));
    }

    #[test]
    fn test_status_config_without_default() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes.web]

[processes.web.status]
transitions = [
  {{ pattern = "Listening", label = "Ready" }},
]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();

        let web_config = config.processes.get("web").unwrap();
        let status = web_config.status.as_ref().unwrap();
        assert_eq!(status.default, None);
        assert_eq!(status.transitions.len(), 1);
        assert_eq!(status.transitions[0].color, None);
    }

    #[test]
    fn test_status_config_missing_is_none() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes.worker]
log_file = "logs/worker.log"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();

        let worker_config = config.processes.get("worker").unwrap();
        assert!(worker_config.status.is_none());
    }

    #[test]
    fn test_status_config_roundtrip() {
        let mut processes = HashMap::new();
        processes.insert(
            "webpack".to_string(),
            ProcessConfig {
                log_file: Some(PathBuf::from("logs/webpack.log")),
                status: Some(StatusConfig {
                    default: Some("Preparing".to_string()),
                    color: None,
                    transitions: vec![
                        StatusTransition {
                            pattern: "webpack compiled".to_string(),
                            label: "Ready".to_string(),
                            color: Some("green".to_string()),
                        },
                        StatusTransition {
                            pattern: "Compiling".to_string(),
                            label: "Building".to_string(),
                            color: None,
                        },
                    ],
                }),
            },
        );

        let original = Config {
            processes,
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();

        let webpack = loaded.processes.get("webpack").unwrap();
        let status = webpack.status.as_ref().unwrap();
        assert_eq!(status.default, Some("Preparing".to_string()));
        assert_eq!(status.transitions.len(), 2);
        assert_eq!(status.transitions[0].label, "Ready");
        assert_eq!(status.transitions[1].label, "Building");
    }

    #[test]
    fn test_status_config_not_serialized_when_none() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_string.contains("status"),
            "status should not be serialized when None"
        );
    }

    #[test]
    fn test_status_config_empty_transitions() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes.api]

[processes.api.status]
default = "Starting"
transitions = []
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();

        let api_config = config.processes.get("api").unwrap();
        let status = api_config.status.as_ref().unwrap();
        assert_eq!(status.default, Some("Starting".to_string()));
        assert!(status.transitions.is_empty());
    }

    #[test]
    fn test_log_files_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[[log_files]]
name = "rails"
path = "log/development.log"

[[log_files]]
name = "sidekiq"
path = "log/sidekiq.log"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.log_files.len(), 2);
        assert_eq!(config.log_files[0].name, "rails");
        assert_eq!(config.log_files[0].path, PathBuf::from("log/development.log"));
        assert_eq!(config.log_files[1].name, "sidekiq");
        assert_eq!(config.log_files[1].path, PathBuf::from("log/sidekiq.log"));
    }

    #[test]
    fn test_log_files_defaults_to_empty() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.log_files.is_empty());
    }

    #[test]
    fn test_log_files_roundtrip() {
        let config = Config {
            log_files: vec![
                LogFileConfig {
                    name: "rails".to_string(),
                    path: PathBuf::from("log/development.log"),
                },
                LogFileConfig {
                    name: "sidekiq".to_string(),
                    path: PathBuf::from("log/sidekiq.log"),
                },
            ],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.log_files.len(), 2);
        assert_eq!(loaded.log_files[0].name, "rails");
        assert_eq!(loaded.log_files[1].name, "sidekiq");
    }

    #[test]
    fn test_validate_passes_with_no_conflicts() {
        let config = Config {
            log_files: vec![LogFileConfig {
                name: "rails".to_string(),
                path: PathBuf::from("log/development.log"),
            }],
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        assert!(config.validate(&process_names).is_ok());
    }

    #[test]
    fn test_validate_fails_on_name_collision() {
        let config = Config {
            log_files: vec![LogFileConfig {
                name: "web".to_string(),
                path: PathBuf::from("log/web.log"),
            }],
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("conflicts with a process name"));
    }

    #[test]
    fn test_validate_fails_on_duplicate_log_file_name() {
        let config = Config {
            log_files: vec![
                LogFileConfig {
                    name: "rails".to_string(),
                    path: PathBuf::from("log/development.log"),
                },
                LogFileConfig {
                    name: "rails".to_string(),
                    path: PathBuf::from("log/other.log"),
                },
            ],
            ..test_config()
        };

        let process_names = vec!["web".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate log file name"));
    }

    #[test]
    fn test_colors_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[colors]
web = "green"
worker = "yellow"
rails = "cyan"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.colors.len(), 3);
        assert_eq!(config.colors.get("web"), Some(&"green".to_string()));
        assert_eq!(config.colors.get("worker"), Some(&"yellow".to_string()));
        assert_eq!(config.colors.get("rails"), Some(&"cyan".to_string()));
    }

    #[test]
    fn test_colors_defaults_to_empty() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.colors.is_empty());
    }

    #[test]
    fn test_colors_roundtrip() {
        let mut colors = HashMap::new();
        colors.insert("web".to_string(), "green".to_string());
        colors.insert("worker".to_string(), "yellow".to_string());

        let config = Config {
            colors,
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.colors.len(), 2);
        assert_eq!(loaded.colors.get("web"), Some(&"green".to_string()));
        assert_eq!(loaded.colors.get("worker"), Some(&"yellow".to_string()));
    }

    #[test]
    fn test_process_coloring_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
process_coloring = true

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.process_coloring, Some(true));
    }

    #[test]
    fn test_process_coloring_defaults_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.process_coloring, None);
    }

    #[test]
    fn test_process_coloring_none_not_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_string.contains("process_coloring"),
            "process_coloring should not be serialized when None"
        );
    }

    #[test]
    fn test_process_coloring_some_is_serialized() {
        let config = Config {
            process_coloring: Some(true),
            ..test_config()
        };

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("process_coloring = true"),
            "process_coloring should be serialized when Some(true)"
        );
    }

    #[test]
    fn test_process_coloring_roundtrip() {
        let config = Config {
            process_coloring: Some(true),
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.process_coloring, Some(true));
    }

    #[test]
    fn test_context_copy_seconds_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
context_copy_seconds = 2.5

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.context_copy_seconds, Some(2.5));
    }

    #[test]
    fn test_context_copy_seconds_defaults_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.context_copy_seconds, None);
    }

    #[test]
    fn test_context_copy_seconds_none_not_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            !toml_string.contains("context_copy_seconds"),
            "context_copy_seconds should not be serialized when None"
        );
    }

    #[test]
    fn test_context_copy_seconds_some_is_serialized() {
        let config = Config {
            context_copy_seconds: Some(1.5),
            ..test_config()
        };

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("context_copy_seconds = 1.5"),
            "context_copy_seconds should be serialized when Some(1.5)"
        );
    }

    #[test]
    fn test_context_copy_seconds_roundtrip() {
        let config = Config {
            context_copy_seconds: Some(3.0),
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.context_copy_seconds, Some(3.0));
    }

    #[test]
    fn test_start_processes_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
start_processes = ["web", "worker"]

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.start_processes, vec!["web", "worker"]);
    }

    #[test]
    fn test_start_processes_defaults_to_empty_when_missing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[processes]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.start_processes.is_empty());
    }

    #[test]
    fn test_start_processes_saves_to_config() {
        let config = Config {
            start_processes: vec!["api".to_string(), "db".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded_config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded_config.start_processes, vec!["api", "db"]);
    }

    #[test]
    fn test_start_processes_empty_array_serialized() {
        let config = test_config();

        let toml_string = toml::to_string_pretty(&config).unwrap();
        assert!(
            toml_string.contains("start_processes = []"),
            "empty start_processes should be serialized as empty array"
        );
    }

    #[test]
    fn test_start_processes_roundtrip() {
        let original = Config {
            start_processes: vec!["web".to_string(), "worker".to_string(), "scheduler".to_string()],
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        original.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.start_processes, original.start_processes);
    }

    #[test]
    fn test_validate_passes_with_valid_start_processes() {
        let config = Config {
            start_processes: vec!["web".to_string(), "worker".to_string()],
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string(), "scheduler".to_string()];
        assert!(config.validate(&process_names).is_ok());
    }

    #[test]
    fn test_validate_fails_with_unknown_start_process() {
        let config = Config {
            start_processes: vec!["web".to_string(), "unknown".to_string()],
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start_processes contains unknown process"));
    }

    #[test]
    fn test_groups_loads_from_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"

[groups]
rails = ["puma", "workers"]
frontend = ["webpack", "esbuild"]
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.groups.len(), 2);
        assert_eq!(config.groups.get("rails"), Some(&vec!["puma".to_string(), "workers".to_string()]));
        assert_eq!(config.groups.get("frontend"), Some(&vec!["webpack".to_string(), "esbuild".to_string()]));
    }

    #[test]
    fn test_groups_defaults_to_empty() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
procfile = "Procfile"
"#
        )
        .unwrap();

        let config = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert!(config.groups.is_empty());
    }

    #[test]
    fn test_groups_roundtrip() {
        let mut groups = HashMap::new();
        groups.insert("rails".to_string(), vec!["puma".to_string(), "workers".to_string()]);
        groups.insert("frontend".to_string(), vec!["webpack".to_string()]);

        let config = Config {
            groups,
            ..test_config()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path().to_str().unwrap()).unwrap();

        let loaded = Config::from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(loaded.groups.len(), 2);
        assert_eq!(loaded.groups.get("rails"), Some(&vec!["puma".to_string(), "workers".to_string()]));
        assert_eq!(loaded.groups.get("frontend"), Some(&vec!["webpack".to_string()]));
    }

    #[test]
    fn test_validate_passes_with_valid_groups() {
        let mut groups = HashMap::new();
        groups.insert("rails".to_string(), vec!["web".to_string(), "worker".to_string()]);

        let config = Config {
            groups,
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string(), "scheduler".to_string()];
        assert!(config.validate(&process_names).is_ok());
    }

    #[test]
    fn test_validate_fails_when_group_name_conflicts_with_process() {
        let mut groups = HashMap::new();
        groups.insert("web".to_string(), vec!["worker".to_string()]);

        let config = Config {
            groups,
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Group name 'web' conflicts with a process name"));
    }

    #[test]
    fn test_validate_fails_when_group_name_conflicts_with_log_file() {
        let mut groups = HashMap::new();
        groups.insert("rails".to_string(), vec!["web".to_string()]);

        let config = Config {
            log_files: vec![LogFileConfig {
                name: "rails".to_string(),
                path: PathBuf::from("log/development.log"),
            }],
            groups,
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Group name 'rails' conflicts with a log file name"));
    }

    #[test]
    fn test_validate_fails_when_group_contains_unknown_process() {
        let mut groups = HashMap::new();
        groups.insert("rails".to_string(), vec!["web".to_string(), "unknown".to_string()]);

        let config = Config {
            groups,
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Group 'rails' contains unknown process 'unknown'"));
    }

    #[test]
    fn test_validate_fails_when_group_is_empty() {
        let mut groups = HashMap::new();
        groups.insert("empty".to_string(), vec![]);

        let config = Config {
            groups,
            ..test_config()
        };

        let process_names = vec!["web".to_string(), "worker".to_string()];
        let result = config.validate(&process_names);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Group 'empty' cannot be empty"));
    }
}
