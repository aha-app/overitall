use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::ui::App;

/// Toggle compact mode and persist to config.
/// Returns the new mode name ("compact" or "full").
pub fn toggle_compact_mode(app: &mut App, config: &mut Config) -> String {
    app.toggle_compact_mode();
    config.compact_mode = Some(app.compact_mode);
    save_config_with_error(config, app);
    if app.compact_mode { "compact" } else { "full" }.to_string()
}
