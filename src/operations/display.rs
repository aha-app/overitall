use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::ui::App;

/// Cycle display mode and persist to config.
/// Returns the new mode name ("compact", "full", or "wrap").
pub fn cycle_display_mode(app: &mut App, config: &mut Config) -> String {
    app.cycle_display_mode();
    // Store in config for persistence (bool for backwards compat: true = compact, false = non-compact)
    config.compact_mode = Some(app.is_compact());
    save_config_with_error(config, app);
    app.display_mode.name().to_string()
}
