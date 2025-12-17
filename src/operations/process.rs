use crate::process::ProcessManager;

/// Start a stopped process.
/// Returns Ok with success message or Err with error message.
pub async fn start_process(manager: &mut ProcessManager, name: &str) -> Result<String, String> {
    match manager.start_process(name).await {
        Ok(_) => Ok(format!("Started process: {}", name)),
        Err(e) => Err(format!("Failed to start {}: {}", name, e)),
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
