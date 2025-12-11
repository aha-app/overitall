use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// Re-export log types for compatibility
pub use crate::log::{LogLine, LogSource};
use crate::log::{LogBuffer, FileReader};

/// Status of a managed process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Terminating,
    Failed(String),
}

/// Handle for a single managed process
pub struct ProcessHandle {
    pub name: String,
    pub command: String,
    pub status: ProcessStatus,
    child: Option<Child>,
    stdout_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
}

impl ProcessHandle {
    /// Create a new process handle (not yet started)
    pub fn new(name: String, command: String) -> Self {
        Self {
            name,
            command,
            status: ProcessStatus::Stopped,
            child: None,
            stdout_task: None,
            stderr_task: None,
        }
    }

    /// Start the process and capture its output
    pub async fn start(&mut self, log_tx: mpsc::UnboundedSender<LogLine>) -> Result<()> {
        if self.status == ProcessStatus::Running {
            return Ok(());
        }

        // Parse command into parts (simple split on whitespace for now)
        let parts: Vec<&str> = self.command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // Spawn with piped stdout/stderr
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn process")?;

        // Capture stdout
        let stdout = child.stdout.take().unwrap();
        let name = self.name.clone();
        let tx = log_tx.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogLine::new(LogSource::ProcessStdout(name.clone()), line));
            }
        });

        // Capture stderr
        let stderr = child.stderr.take().unwrap();
        let name = self.name.clone();
        let tx = log_tx;
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogLine::new(LogSource::ProcessStderr(name.clone()), line));
            }
        });

        self.child = Some(child);
        self.stdout_task = Some(stdout_task);
        self.stderr_task = Some(stderr_task);
        self.status = ProcessStatus::Running;

        Ok(())
    }

    pub async fn kill(&mut self) -> Result<()> {
        if let Some(child) = &mut self.child {
            // Set to Terminating status first
            self.status = ProcessStatus::Terminating;

            // Send kill signal
            child.kill().await.context("Failed to kill process")?;

            // Cancel the output capture tasks
            if let Some(task) = self.stdout_task.take() {
                task.abort();
            }
            if let Some(task) = self.stderr_task.take() {
                task.abort();
            }

            // Don't set to Stopped immediately - let check_status do that
            // This allows the UI to show "Terminating" status
        }
        Ok(())
    }

    /// Check if the process has actually exited after being killed
    pub async fn is_terminated(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    self.status = ProcessStatus::Stopped;
                    self.child = None;
                    true
                }
                Ok(None) => {
                    // Still terminating
                    false
                }
                Err(_) => {
                    // Error checking status, assume terminated
                    self.status = ProcessStatus::Stopped;
                    self.child = None;
                    true
                }
            }
        } else {
            // No child process, already terminated
            true
        }
    }

    /// Check if process has exited
    pub async fn check_status(&mut self) -> ProcessStatus {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        self.status = ProcessStatus::Stopped;
                    } else {
                        self.status = ProcessStatus::Failed(format!("Exit code: {:?}", status.code()));
                    }
                    self.child = None;
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    self.status = ProcessStatus::Failed(e.to_string());
                    self.child = None;
                }
            }
        }
        self.status.clone()
    }

    /// Restart the process (kill then start)
    pub async fn restart(&mut self, log_tx: mpsc::UnboundedSender<LogLine>) -> Result<()> {
        self.kill().await?;
        self.start(log_tx).await?;
        Ok(())
    }
}

/// Manages multiple processes
pub struct ProcessManager {
    processes: HashMap<String, ProcessHandle>,
    log_sources: Vec<FileReader>,
    log_buffer: LogBuffer,
    log_tx: mpsc::UnboundedSender<LogLine>,
    log_rx: mpsc::UnboundedReceiver<LogLine>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        Self {
            processes: HashMap::new(),
            log_sources: Vec::new(),
            log_buffer: LogBuffer::new_default(),
            log_tx,
            log_rx,
        }
    }

    /// Add a process definition (doesn't start it)
    pub fn add_process(&mut self, name: String, command: String) {
        self.processes.insert(name.clone(), ProcessHandle::new(name, command));
    }

    pub async fn start_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.start(self.log_tx.clone()).await
    }

    pub async fn start_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.processes.keys().cloned().collect();
        for name in names {
            self.start_process(&name).await?;
        }
        Ok(())
    }

    pub async fn kill_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.kill().await
    }

    pub async fn restart_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.restart(self.log_tx.clone()).await
    }

    pub async fn kill_all(&mut self) -> Result<()> {
        for process in self.processes.values_mut() {
            let _ = process.kill().await; // Ignore errors during shutdown
        }
        Ok(())
    }

    /// Check if all processes have finished terminating
    pub async fn check_termination_status(&mut self) -> bool {
        let mut all_terminated = true;
        for process in self.processes.values_mut() {
            if !process.is_terminated().await {
                all_terminated = false;
            }
        }
        all_terminated
    }

    pub async fn check_all_status(&mut self) {
        for process in self.processes.values_mut() {
            process.check_status().await;
        }
    }

    pub fn get_status(&self, name: &str) -> Option<ProcessStatus> {
        self.processes.get(name).map(|p| p.status.clone())
    }

    pub fn get_all_statuses(&self) -> Vec<(String, ProcessStatus)> {
        self.processes
            .iter()
            .map(|(name, handle)| (name.clone(), handle.status.clone()))
            .collect()
    }

    /// Add a log file to tail for a specific process
    pub async fn add_log_file(&mut self, process_name: String, path: PathBuf) -> Result<()> {
        let mut reader = FileReader::new(process_name, path);
        reader.start(self.log_tx.clone()).await?;
        self.log_sources.push(reader);
        Ok(())
    }

    /// Process incoming logs from the channel into the buffer
    pub fn process_logs(&mut self) {
        while let Ok(log) = self.log_rx.try_recv() {
            self.log_buffer.push(log);
        }
    }

    pub fn get_recent_logs(&self, n: usize) -> Vec<&LogLine> {
        self.log_buffer.get_last(n)
    }

    pub fn get_all_logs(&self) -> Vec<&LogLine> {
        self.log_buffer.get_all()
    }

    /// Try to receive a log line (non-blocking)
    pub fn try_recv_log(&mut self) -> Option<LogLine> {
        self.log_rx.try_recv().ok()
    }

    /// Receive a log line (blocking)
    pub async fn recv_log(&mut self) -> Option<LogLine> {
        self.log_rx.recv().await
    }

    /// Add a log line directly to the buffer (for testing)
    pub fn add_test_log(&mut self, log: LogLine) {
        self.log_buffer.push(log);
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        // Kill all processes when manager is dropped
        for process in self.processes.values_mut() {
            if let Some(child) = &mut process.child {
                let _ = child.start_kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_start_stop() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "echo hello".to_string());

        manager.start_process("test").await.unwrap();
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Running));

        // Give it a moment to run
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        manager.kill_process("test").await.unwrap();
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Stopped));
    }
}
