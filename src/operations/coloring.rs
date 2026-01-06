use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::process::ProcessManager;
use crate::ui::App;
use std::collections::HashMap;

/// Toggle process coloring on/off.
/// Returns true if coloring is now enabled, false if disabled.
pub fn toggle_coloring(app: &mut App, manager: &ProcessManager, config: &mut Config) -> bool {
    if app.display.coloring_enabled {
        disable_coloring(app, config)
    } else {
        enable_coloring(app, manager, config)
    }
}

/// Enable process coloring.
/// Returns true to indicate coloring is now enabled.
fn enable_coloring(app: &mut App, manager: &ProcessManager, config: &mut Config) -> bool {
    let process_names: Vec<String> = manager.get_processes().keys().cloned().collect();
    let log_file_names = manager.get_standalone_log_file_names();
    app.init_process_colors(&process_names, &log_file_names, &config.colors);
    app.display.coloring_enabled = true;

    // Persist to config
    config.process_coloring = Some(true);
    save_config_with_error(config, app);

    true
}

/// Disable process coloring.
/// Returns false to indicate coloring is now disabled.
fn disable_coloring(app: &mut App, config: &mut Config) -> bool {
    // Reset to empty colors (all will return White)
    app.init_process_colors(&[], &[], &HashMap::new());
    app.display.coloring_enabled = false;

    // Persist to config
    config.process_coloring = Some(false);
    save_config_with_error(config, app);

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        App::new()
    }

    fn create_test_config() -> Config {
        Config {
            procfile: std::path::PathBuf::from("Procfile"),
            processes: std::collections::HashMap::new(),
            log_files: Vec::new(),
            filters: crate::config::FilterConfig::default(),
            batch_window_ms: None,
            max_log_buffer_mb: None,
            hidden_processes: Vec::new(),
            ignored_processes: Vec::new(),
            disable_auto_update: None,
            compact_mode: None,
            colors: std::collections::HashMap::new(),
            process_coloring: None,
            context_copy_seconds: None,
            config_path: None,
        }
    }

    #[test]
    fn test_toggle_from_disabled_enables() {
        let mut app = create_test_app();
        let manager = ProcessManager::new_with_buffer_limit(50);
        let mut config = create_test_config();

        assert!(!app.display.coloring_enabled);

        let result = toggle_coloring(&mut app, &manager, &mut config);

        assert!(result);
        assert!(app.display.coloring_enabled);
        assert_eq!(config.process_coloring, Some(true));
    }

    #[test]
    fn test_toggle_from_enabled_disables() {
        let mut app = create_test_app();
        app.display.coloring_enabled = true;
        let manager = ProcessManager::new_with_buffer_limit(50);
        let mut config = create_test_config();

        let result = toggle_coloring(&mut app, &manager, &mut config);

        assert!(!result);
        assert!(!app.display.coloring_enabled);
        assert_eq!(config.process_coloring, Some(false));
    }

    #[test]
    fn test_double_toggle_returns_to_original() {
        let mut app = create_test_app();
        let manager = ProcessManager::new_with_buffer_limit(50);
        let mut config = create_test_config();

        assert!(!app.display.coloring_enabled);

        toggle_coloring(&mut app, &manager, &mut config);
        assert!(app.display.coloring_enabled);

        toggle_coloring(&mut app, &manager, &mut config);
        assert!(!app.display.coloring_enabled);
    }
}
