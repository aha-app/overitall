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
