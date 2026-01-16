use crate::config::Config;
use crate::operations::config::save_config_with_error;
use crate::process::ProcessManager;
use crate::ui::App;

/// Hide a process or log file's output from the log viewer.
/// Returns an error message if the process/log file doesn't exist.
pub fn hide_process(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config,
    process: &str,
) -> Result<(), String> {
    if manager.has_process(process) || manager.has_standalone_log_file(process) {
        app.filters.hidden_processes.insert(process.to_string());
        sync_hidden_processes_to_config(app, config);
        save_config_with_error(config, app);
        Ok(())
    } else {
        Err(format!("Process or log file not found: {}", process))
    }
}

/// Show a process's output in the log viewer.
/// Returns Ok(true) if the process was hidden and is now shown.
/// Returns Ok(false) if the process was not hidden.
pub fn show_process(app: &mut App, config: &mut Config, process: &str) -> Result<bool, ()> {
    let was_hidden = app.filters.hidden_processes.remove(process);
    if was_hidden {
        sync_hidden_processes_to_config(app, config);
        save_config_with_error(config, app);
    }
    Ok(was_hidden)
}

/// Hide all processes and log files output from the log viewer.
pub fn hide_all(app: &mut App, manager: &ProcessManager, config: &mut Config) {
    let all_processes: Vec<String> = manager.get_processes().keys().cloned().collect();
    for process in all_processes {
        app.filters.hidden_processes.insert(process);
    }
    for log_file in manager.get_standalone_log_file_names() {
        app.filters.hidden_processes.insert(log_file);
    }
    sync_hidden_processes_to_config(app, config);
    save_config_with_error(config, app);
}

/// Show all processes' output in the log viewer.
/// Returns the count of processes that were hidden.
pub fn show_all(app: &mut App, config: &mut Config) -> usize {
    let count = app.filters.hidden_processes.len();
    app.filters.hidden_processes.clear();
    sync_hidden_processes_to_config(app, config);
    save_config_with_error(config, app);
    count
}

/// Show only a single process or log file, hiding all others.
/// Returns an error message if the process/log file doesn't exist.
pub fn only_process(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config,
    process: &str,
) -> Result<(), String> {
    only_processes(app, manager, config, &[process.to_string()])
}

/// Show only the specified processes or log files, hiding all others.
/// Returns an error message if any process/log file doesn't exist.
pub fn only_processes(
    app: &mut App,
    manager: &ProcessManager,
    config: &mut Config,
    processes: &[String],
) -> Result<(), String> {
    use std::collections::HashSet;

    let show_set: HashSet<&str> = processes.iter().map(|s| s.as_str()).collect();

    for process in &show_set {
        if !manager.has_process(process) && !manager.has_standalone_log_file(process) {
            return Err(format!("Process or log file not found: {}", process));
        }
    }

    let all_processes: Vec<String> = manager.get_processes().keys().cloned().collect();
    let all_log_files = manager.get_standalone_log_file_names();
    app.filters.hidden_processes.clear();
    for p in all_processes {
        if !show_set.contains(p.as_str()) {
            app.filters.hidden_processes.insert(p);
        }
    }
    for lf in all_log_files {
        if !show_set.contains(lf.as_str()) {
            app.filters.hidden_processes.insert(lf);
        }
    }
    sync_hidden_processes_to_config(app, config);
    save_config_with_error(config, app);
    Ok(())
}

/// Sync the app's hidden_processes set to the config.
fn sync_hidden_processes_to_config(app: &App, config: &mut Config) {
    config.hidden_processes = app.filters.hidden_processes.iter().cloned().collect();
}
