use crate::process::ProcessManager;

/// Start a stopped process.
/// Returns Ok with success message or Err with error message.
pub async fn start_process(manager: &mut ProcessManager, name: &str) -> Result<String, String> {
    match manager.start_process(name).await {
        Ok(_) => Ok(format!("Started process: {}", name)),
        Err(e) => Err(format!("Failed to start {}: {}", name, e)),
    }
}

/// Restart a running or stopped process.
/// Returns Ok with success message or Err with error message.
pub async fn restart_process(manager: &mut ProcessManager, name: &str) -> Result<String, String> {
    match manager.restart_process(name).await {
        Ok(_) => Ok(format!("Restarted process: {}", name)),
        Err(e) => Err(format!("Failed to restart {}: {}", name, e)),
    }
}

/// Kill a running process.
/// Returns Ok with success message or Err with error message.
pub async fn kill_process(manager: &mut ProcessManager, name: &str) -> Result<String, String> {
    match manager.kill_process(name).await {
        Ok(_) => Ok(format!("Killed process: {}", name)),
        Err(e) => Err(format!("Failed to kill {}: {}", name, e)),
    }
}

/// Restart all processes.
/// Returns Ok with success message or Err with error message.
pub async fn restart_all_processes(manager: &mut ProcessManager) -> Result<String, String> {
    let names: Vec<String> = manager.get_all_statuses().into_iter().map(|(n, _)| n).collect();
    if names.is_empty() {
        return Err("No processes to restart".to_string());
    }

    let mut restarted = Vec::new();
    let mut failed = Vec::new();

    for name in &names {
        match manager.restart_process(name).await {
            Ok(_) => restarted.push(name.clone()),
            Err(e) => failed.push(format!("{}: {}", name, e)),
        }
    }

    if failed.is_empty() {
        Ok(format!("Restarted {} process(es): {}", restarted.len(), restarted.join(", ")))
    } else if restarted.is_empty() {
        Err(format!("Failed to restart all processes: {}", failed.join("; ")))
    } else {
        Ok(format!(
            "Restarted: {}. Failed: {}",
            restarted.join(", "),
            failed.join("; ")
        ))
    }
}
