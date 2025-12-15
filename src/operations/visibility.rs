use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::process::ProcessManager;
use crate::ui::App;

/// Hide a process's output from the log viewer.
/// Returns an error message if the process doesn't exist.
pub fn hide_process(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config,
    process: &str,
) -> Result<(), String> {
    if manager.has_process(process) {
        app.hidden_processes.insert(process.to_string());
        sync_hidden_processes_to_config(app, config);
        save_config_with_error(config, app);
        Ok(())
    } else {
        Err(format!("Process not found: {}", process))
    }
}

/// Show a process's output in the log viewer.
/// Returns Ok(true) if the process was hidden and is now shown.
/// Returns Ok(false) if the process was not hidden.
pub fn show_process(app: &mut App, config: &mut Config, process: &str) -> Result<bool, ()> {
    let was_hidden = app.hidden_processes.remove(process);
    if was_hidden {
        sync_hidden_processes_to_config(app, config);
        save_config_with_error(config, app);
    }
    Ok(was_hidden)
}

/// Hide all processes' output from the log viewer.
pub fn hide_all(app: &mut App, manager: &ProcessManager, config: &mut Config) {
    let all_processes: Vec<String> = manager.get_processes().keys().cloned().collect();
    for process in all_processes {
        app.hidden_processes.insert(process);
    }
    sync_hidden_processes_to_config(app, config);
    save_config_with_error(config, app);
}

/// Show all processes' output in the log viewer.
/// Returns the count of processes that were hidden.
pub fn show_all(app: &mut App, config: &mut Config) -> usize {
    let count = app.hidden_processes.len();
    app.hidden_processes.clear();
    sync_hidden_processes_to_config(app, config);
    save_config_with_error(config, app);
    count
}

/// Sync the app's hidden_processes set to the config.
fn sync_hidden_processes_to_config(app: &App, config: &mut Config) {
    config.hidden_processes = app.hidden_processes.iter().cloned().collect();
}
