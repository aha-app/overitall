use crate::config::Config;
use crate::ui::App;

/// Save config to file and surface any errors to the status bar.
/// This consolidates the common pattern of saving config with error handling.
pub fn save_config_with_error(config: &Config, app: &mut App) {
    if let Some(path) = &config.config_path {
        if let Err(e) = config.save_to_file(path) {
            app.set_status_error(format!("Config save failed: {}", e));
        }
    }
}
