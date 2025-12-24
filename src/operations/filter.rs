use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::ui::{App, FilterType};

/// Add an include filter and save to config.
pub fn add_include_filter(app: &mut App, config: &mut Config, pattern: String) {
    app.filters.add_include_filter(pattern);
    config.update_filters(&app.filters.filters);
    save_config_with_error(config, app);
}

/// Add an exclude filter and save to config.
pub fn add_exclude_filter(app: &mut App, config: &mut Config, pattern: String) {
    app.filters.add_exclude_filter(pattern);
    config.update_filters(&app.filters.filters);
    save_config_with_error(config, app);
}

/// Clear all filters and save to config. Returns the number of filters that were cleared.
pub fn clear_filters(app: &mut App, config: &mut Config) -> usize {
    let count = app.filters.filter_count();
    app.filters.clear_filters();
    config.update_filters(&app.filters.filters);
    save_config_with_error(config, app);
    count
}

/// Remove a filter by pattern and save to config. Returns true if a filter was removed.
pub fn remove_filter(app: &mut App, config: &mut Config, pattern: &str) -> bool {
    let removed = app.filters.remove_filter(pattern);
    if removed {
        config.update_filters(&app.filters.filters);
        save_config_with_error(config, app);
    }
    removed
}

/// Format the list of current filters for display.
/// Returns None if there are no filters, otherwise returns a formatted string.
pub fn list_filters(app: &App) -> Option<String> {
    if app.filters.filters.is_empty() {
        None
    } else {
        let filter_strs: Vec<String> = app
            .filters
            .filters
            .iter()
            .map(|f| {
                let type_str = match f.filter_type {
                    FilterType::Include => "include",
                    FilterType::Exclude => "exclude",
                };
                format!("{}: {}", type_str, f.pattern)
            })
            .collect();
        Some(format!("Filters: {}", filter_strs.join(", ")))
    }
}
