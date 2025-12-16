use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub procfile: PathBuf,
    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,
    #[serde(default)]
    pub filters: FilterConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_log_buffer_mb: Option<usize>,
    #[serde(default)]
    pub hidden_processes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_mode: Option<bool>,

    // This field is not serialized, just used at runtime
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,
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
            filters: FilterConfig::default(),
            batch_window_ms: None,
            max_log_buffer_mb: None,
            hidden_processes: Vec::new(),
            disable_auto_update: None,
            compact_mode: None,
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
}
