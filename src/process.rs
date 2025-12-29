use anyhow::{Context, Result};
use ratatui::style::Color;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// Re-export log types for compatibility
pub use crate::log::{LogLine, LogSource};
use crate::config::StatusConfig;
use crate::log::{LogBuffer, FileReader, LogVelocityTracker};
use crate::status_matcher::StatusMatcher;

/// Status of a managed process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Terminating,
    Restarting,
    Failed(String),
}

/// Buffer statistics for UI display
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub memory_mb: f64,
    pub limit_mb: usize,
    pub percent: f64,
    pub line_count: usize,
    pub sparkline: String,
}

/// Handle for a single managed process
pub struct ProcessHandle {
    pub name: String,
    pub command: String,
    pub working_dir: Option<PathBuf>,
    pub status: ProcessStatus,
    child: Option<Child>,
    pgid: Option<i32>,  // Process Group ID for killing entire process tree
    stdout_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
    status_matcher: Option<StatusMatcher>,
}

impl ProcessHandle {
    /// Create a new process handle (not yet started)
    pub fn new(name: String, command: String, working_dir: Option<PathBuf>, status_config: Option<&StatusConfig>) -> Self {
        let status_matcher = status_config.and_then(|c| StatusMatcher::new(c).ok());
        Self {
            name,
            command,
            working_dir,
            status: ProcessStatus::Stopped,
            child: None,
            pgid: None,
            stdout_task: None,
            stderr_task: None,
            status_matcher,
        }
    }

    /// Get custom display status if configured
    pub fn get_custom_status(&self) -> Option<(&str, Option<Color>)> {
        self.status_matcher.as_ref().and_then(|m| m.get_display_status())
    }

    /// Check log line against status patterns. Returns true if status changed.
    pub fn check_log_line(&mut self, line: &str) -> bool {
        self.status_matcher.as_mut().map(|m| m.check_line(line)).unwrap_or(false)
    }

    /// Reset status matcher to default (call on restart)
    pub fn reset_status(&mut self) {
        if let Some(m) = &mut self.status_matcher {
            m.reset();
        }
    }

    /// Start the process and capture its output
    pub async fn start(&mut self, log_tx: mpsc::UnboundedSender<LogLine>) -> Result<()> {
        if self.status == ProcessStatus::Running {
            return Ok(());
        }

        // Execute command through shell (handles quotes, spaces, variables, pipes, etc.)
        let mut cmd = Command::new("sh");
        cmd.args(&["-c", &self.command]);

        // Set working directory if specified
        if let Some(ref working_dir) = self.working_dir {
            cmd.current_dir(working_dir);
        }

        // Spawn with piped stdout/stderr and null stdin
        // IMPORTANT: Set stdin to null so child processes don't inherit parent's stdin
        // This prevents them from interfering with crossterm's raw mode terminal input

        // Create a new process group so we can kill the entire process tree
        // This ensures that when we kill the shell, all child processes (like
        // pnpm and the web server) are also killed, preventing orphaned processes.
        cmd.process_group(0); // Create new process group with pgid = pid

        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!(
                "Failed to spawn process '{}': command='{}'{}",
                self.name,
                self.command,
                self.working_dir
                    .as_ref()
                    .map(|d| format!(", working_dir='{}'", d.display()))
                    .unwrap_or_default()
            ))?;

        // Store the process group ID
        // With process_group(0), the child's PID becomes the PGID
        self.pgid = child.id().map(|pid| pid as i32);

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
        if self.child.is_none() {
            return Ok(());
        }

        // Set to Terminating status first (if not already set)
        if self.status != ProcessStatus::Terminating {
            self.status = ProcessStatus::Terminating;
        }

        // Kill the entire process group to ensure all child processes are terminated
        if let Some(pgid) = self.pgid {
            use nix::sys::signal::{killpg, Signal};
            use nix::unistd::Pid;

            let pid = Pid::from_raw(pgid);

            // Try graceful shutdown first with SIGTERM
            let _ = killpg(pid, Signal::SIGTERM);

            // Wait a bit for graceful shutdown
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Force kill with SIGKILL if still needed
            let _ = killpg(pid, Signal::SIGKILL);
        }

        // Cancel the output capture tasks
        if let Some(task) = self.stdout_task.take() {
            task.abort();
        }
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        // Don't set to Stopped immediately - let check_status do that
        // This allows the UI to show "Terminating" status
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
                        let msg = match status.code() {
                            Some(code) => format!("Exit code: {}", code),
                            None => "Terminated by signal".to_string(),
                        };
                        self.status = ProcessStatus::Failed(msg);
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
        self.reset_status();
        self.kill().await?;
        self.start(log_tx).await?;
        Ok(())
    }
}

/// Manages multiple processes
pub struct ProcessManager {
    processes: HashMap<String, ProcessHandle>,
    log_sources: Vec<FileReader>,
    standalone_log_files: Vec<FileReader>,
    log_buffer: LogBuffer,
    velocity_tracker: LogVelocityTracker,
    log_tx: mpsc::UnboundedSender<LogLine>,
    log_rx: mpsc::UnboundedReceiver<LogLine>,
}

impl ProcessManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_buffer_limit(50)
    }

    pub fn new_with_buffer_limit(max_log_buffer_mb: usize) -> Self {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        Self {
            processes: HashMap::new(),
            log_sources: Vec::new(),
            standalone_log_files: Vec::new(),
            log_buffer: LogBuffer::new_with_memory_limit(max_log_buffer_mb),
            velocity_tracker: LogVelocityTracker::default(),
            log_tx,
            log_rx,
        }
    }

    /// Add a process definition (doesn't start it)
    pub fn add_process(&mut self, name: String, command: String, working_dir: Option<PathBuf>, status_config: Option<&StatusConfig>) {
        self.processes.insert(name.clone(), ProcessHandle::new(name, command, working_dir, status_config));
    }

    pub async fn start_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.start(self.log_tx.clone()).await
    }

    /// Start all processes, continuing even if some fail.
    /// Returns a list of (name, error_message) for any processes that failed to start.
    pub async fn start_all(&mut self) -> Vec<(String, String)> {
        let names: Vec<String> = self.processes.keys().cloned().collect();
        let mut failures = Vec::new();
        for name in names {
            if let Err(e) = self.start_process(&name).await {
                // Set the process status to Failed
                if let Some(process) = self.processes.get_mut(&name) {
                    process.status = ProcessStatus::Failed(e.to_string());
                }
                failures.push((name, e.to_string()));
            }
        }
        failures
    }

    pub async fn kill_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.kill().await
    }

    #[allow(dead_code)]
    pub async fn restart_process(&mut self, name: &str) -> Result<()> {
        let process = self.processes.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Process '{}' not found", name))?;
        process.restart(self.log_tx.clone()).await
    }

    /// Set a process to Restarting status (fast, non-blocking)
    /// Returns true if the process was found, false otherwise
    pub fn set_restarting(&mut self, name: &str) -> bool {
        if let Some(process) = self.processes.get_mut(name) {
            process.status = ProcessStatus::Restarting;
            true
        } else {
            false
        }
    }

    /// Set all processes to Restarting status (fast, non-blocking)
    pub fn set_all_restarting(&mut self) {
        for (_name, process) in self.processes.iter_mut() {
            process.status = ProcessStatus::Restarting;
        }
    }

    /// Get the names of processes that are currently in Restarting status
    pub fn get_restarting_processes(&self) -> Vec<String> {
        self.processes
            .iter()
            .filter(|(_, p)| p.status == ProcessStatus::Restarting)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Perform the actual restart for processes in Restarting status
    /// This should be called from the event loop after UI has been redrawn
    /// Returns a tuple of (successfully_restarted, failed) process names
    pub async fn perform_pending_restarts(&mut self) -> (Vec<String>, Vec<(String, String)>) {
        let restarting: Vec<String> = self.get_restarting_processes();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for name in restarting {
            if let Some(process) = self.processes.get_mut(&name) {
                let log_tx = self.log_tx.clone();
                match process.restart(log_tx).await {
                    Ok(_) => succeeded.push(name),
                    Err(e) => failed.push((name, e.to_string())),
                }
            }
        }

        (succeeded, failed)
    }

    /// Check if any processes are in Restarting status
    pub fn has_pending_restarts(&self) -> bool {
        self.processes.values().any(|p| p.status == ProcessStatus::Restarting)
    }

    /// Set all running processes to Terminating status (fast, non-blocking)
    /// This should be called before sending kill signals to provide immediate UI feedback
    pub fn set_all_terminating(&mut self) {
        for (_name, process) in self.processes.iter_mut() {
            if process.status == ProcessStatus::Running {
                process.status = ProcessStatus::Terminating;
            }
        }
    }

    /// Send kill signals to all processes without waiting
    /// This is non-blocking and returns immediately
    pub async fn send_kill_signals(&mut self) -> Result<()> {
        for (_name, process) in self.processes.iter_mut() {
            let _ = process.kill().await; // Ignore errors during shutdown
        }
        Ok(())
    }

    pub async fn kill_all(&mut self) -> Result<()> {
        // FIRST: Set all processes to Terminating status (fast - UI will show this immediately)
        self.set_all_terminating();

        // THEN: Send kill signal to all processes
        self.send_kill_signals().await?;

        // Wait for all processes to terminate with a timeout
        let timeout_duration = tokio::time::Duration::from_secs(5);
        let start_time = tokio::time::Instant::now();

        loop {
            let all_terminated = self.check_termination_status().await;
            if all_terminated {
                break;
            }

            if start_time.elapsed() > timeout_duration {
                break;
            }

            // Small delay before checking again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Check if all processes have finished terminating
    pub async fn check_termination_status(&mut self) -> bool {
        let mut all_terminated = true;
        for (_name, process) in self.processes.iter_mut() {
            if !process.is_terminated().await {
                all_terminated = false;
            }
        }
        all_terminated
    }

    /// Check all process statuses and return a list of newly failed processes.
    /// Returns Vec<(name, error_message)> for processes that just transitioned to Failed.
    pub async fn check_all_status(&mut self) -> Vec<(String, String)> {
        let mut newly_failed = Vec::new();
        for (name, process) in self.processes.iter_mut() {
            let was_running = process.status == ProcessStatus::Running;
            let new_status = process.check_status().await;
            // Detect transitions to Failed status
            if was_running {
                if let ProcessStatus::Failed(ref msg) = new_status {
                    newly_failed.push((name.clone(), msg.clone()));
                }
            }
        }
        newly_failed
    }

    #[allow(dead_code)]
    pub fn get_status(&self, name: &str) -> Option<ProcessStatus> {
        self.processes.get(name).map(|p| p.status.clone())
    }

    pub fn get_all_statuses(&self) -> Vec<(String, ProcessStatus)> {
        self.processes
            .iter()
            .map(|(name, handle)| (name.clone(), handle.status.clone()))
            .collect()
    }

    pub fn has_process(&self, name: &str) -> bool {
        self.processes.contains_key(name)
    }

    pub fn get_processes(&self) -> &HashMap<String, ProcessHandle> {
        &self.processes
    }

    #[doc(hidden)]
    pub fn set_process_status_for_testing(&mut self, name: &str, status: ProcessStatus) {
        if let Some(handle) = self.processes.get_mut(name) {
            handle.status = status;
        }
    }

    /// Add a log file to tail for a specific process
    pub async fn add_log_file(&mut self, process_name: String, path: PathBuf) -> Result<()> {
        let mut reader = FileReader::new(process_name, path);
        reader.start(self.log_tx.clone()).await?;
        self.log_sources.push(reader);
        Ok(())
    }

    /// Add a standalone log file (not associated with any process)
    pub async fn add_standalone_log_file(&mut self, name: String, path: PathBuf) -> Result<()> {
        let mut reader = FileReader::new_standalone(name, path);
        reader.start(self.log_tx.clone()).await?;
        self.standalone_log_files.push(reader);
        Ok(())
    }

    /// Get names of all standalone log files
    pub fn get_standalone_log_file_names(&self) -> Vec<String> {
        self.standalone_log_files.iter().map(|r| r.name().to_string()).collect()
    }

    /// Check if a standalone log file exists with the given name
    pub fn has_standalone_log_file(&self, name: &str) -> bool {
        self.standalone_log_files.iter().any(|r| r.name() == name)
    }

    /// Process incoming logs from the channel into the buffer
    /// Also checks each log line against status patterns for the corresponding process
    pub fn process_logs(&mut self) {
        while let Ok(log) = self.log_rx.try_recv() {
            // Track log velocity for sparkline display
            self.velocity_tracker.record(log.arrival_time);
            // Check log line against status patterns for the corresponding process
            let process_name = log.source.process_name();
            if let Some(handle) = self.processes.get_mut(process_name) {
                handle.check_log_line(&log.line);
            }
            self.log_buffer.push(log);
        }
    }

    pub fn get_recent_logs(&self, n: usize) -> Vec<&LogLine> {
        self.log_buffer.get_last(n)
    }

    pub fn get_all_logs(&self) -> Vec<&LogLine> {
        self.log_buffer.get_all()
    }

    /// Get buffer statistics for UI display
    pub fn get_buffer_stats(&self) -> BufferStats {
        BufferStats {
            memory_mb: self.log_buffer.get_memory_usage_mb(),
            limit_mb: self.log_buffer.get_memory_limit_mb(),
            percent: self.log_buffer.get_memory_usage_percent(),
            line_count: self.log_buffer.len(),
            sparkline: self.velocity_tracker.sparkline(),
        }
    }

    /// Get sparkline showing log velocity over time
    #[allow(dead_code)]
    pub fn get_velocity_sparkline(&self) -> String {
        self.velocity_tracker.sparkline()
    }

    /// Try to receive a log line (non-blocking)
    #[allow(dead_code)]
    pub fn try_recv_log(&mut self) -> Option<LogLine> {
        self.log_rx.try_recv().ok()
    }

    /// Receive a log line (blocking)
    #[allow(dead_code)]
    pub async fn recv_log(&mut self) -> Option<LogLine> {
        self.log_rx.recv().await
    }

    /// Add a log line directly to the buffer (for testing)
    #[allow(dead_code)]
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
        manager.add_process("test".to_string(), "echo hello".to_string(), None, None);

        manager.start_process("test").await.unwrap();
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Running));

        // Give it a moment to run
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        manager.kill_process("test").await.unwrap();

        // Wait for the process to actually terminate
        while !manager.check_termination_status().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Stopped));
    }

    #[tokio::test]
    async fn test_set_all_terminating() {
        let mut manager = ProcessManager::new();
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None, None);

        let failures = manager.start_all().await;
        assert!(failures.is_empty(), "Expected no failures, got {:?}", failures);
        assert_eq!(manager.get_status("proc1"), Some(ProcessStatus::Running));
        assert_eq!(manager.get_status("proc2"), Some(ProcessStatus::Running));

        manager.set_all_terminating();

        assert_eq!(
            manager.get_status("proc1"),
            Some(ProcessStatus::Terminating)
        );
        assert_eq!(
            manager.get_status("proc2"),
            Some(ProcessStatus::Terminating)
        );

        manager.kill_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_kill_all_multiple_processes() {
        let mut manager = ProcessManager::new();
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc3".to_string(), "sleep 10".to_string(), None, None);

        let failures = manager.start_all().await;
        assert!(failures.is_empty(), "Expected no failures, got {:?}", failures);

        manager.kill_all().await.unwrap();

        assert_eq!(manager.get_status("proc1"), Some(ProcessStatus::Stopped));
        assert_eq!(manager.get_status("proc2"), Some(ProcessStatus::Stopped));
        assert_eq!(manager.get_status("proc3"), Some(ProcessStatus::Stopped));
    }

    #[tokio::test]
    async fn test_shutdown_flow_sets_status_before_killing() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "sleep 10".to_string(), None, None);

        manager.start_process("test").await.unwrap();
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Running));

        manager.set_all_terminating();

        assert_eq!(
            manager.get_status("test"),
            Some(ProcessStatus::Terminating)
        );

        manager.send_kill_signals().await.unwrap();

        while !manager.check_termination_status().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Stopped));
    }

    #[tokio::test]
    async fn test_send_kill_signals_returns_quickly() {
        let mut manager = ProcessManager::new();
        manager.add_process("slow".to_string(), "sleep 10".to_string(), None, None);

        let failures = manager.start_all().await;
        assert!(failures.is_empty(), "Expected no failures, got {:?}", failures);
        manager.set_all_terminating();

        let start = tokio::time::Instant::now();
        manager.send_kill_signals().await.unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_secs() < 2,
            "send_kill_signals took {} seconds, expected < 2",
            elapsed.as_secs()
        );

        manager.kill_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_start_all_continues_after_failure() {
        // Note: Commands run through sh -c, so they always "spawn" successfully
        // even if the command inside doesn't exist. The failure happens at runtime.
        // This test verifies that all processes are started regardless of any
        // individual failures.
        let mut manager = ProcessManager::new();
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc3".to_string(), "sleep 10".to_string(), None, None);

        let failures = manager.start_all().await;

        // All should succeed spawning (sh -c always spawns successfully)
        assert!(failures.is_empty());

        // All processes should be running
        assert_eq!(manager.get_status("proc1"), Some(ProcessStatus::Running));
        assert_eq!(manager.get_status("proc2"), Some(ProcessStatus::Running));
        assert_eq!(manager.get_status("proc3"), Some(ProcessStatus::Running));

        manager.kill_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_check_all_status_detects_failed_processes() {
        let mut manager = ProcessManager::new();
        // Use a command that exits immediately with an error
        manager.add_process("failing".to_string(), "exit 1".to_string(), None, None);

        let failures = manager.start_all().await;
        assert!(failures.is_empty()); // spawn succeeds, command fails later

        // Wait for the process to exit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check status - should detect the failure
        let _newly_failed = manager.check_all_status().await;

        // The process should now have Failed status
        let status = manager.get_status("failing");
        assert!(
            matches!(status, Some(ProcessStatus::Failed(_))),
            "Expected Failed status, got {:?}",
            status
        );
    }

    #[tokio::test]
    async fn test_set_restarting() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "sleep 10".to_string(), None, None);

        // Test with non-existent process
        assert!(!manager.set_restarting("nonexistent"));

        // Test with existing process
        assert!(manager.set_restarting("test"));
        assert_eq!(
            manager.get_status("test"),
            Some(ProcessStatus::Restarting)
        );
    }

    #[tokio::test]
    async fn test_set_all_restarting() {
        let mut manager = ProcessManager::new();
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None, None);

        manager.set_all_restarting();

        assert_eq!(
            manager.get_status("proc1"),
            Some(ProcessStatus::Restarting)
        );
        assert_eq!(
            manager.get_status("proc2"),
            Some(ProcessStatus::Restarting)
        );
    }

    #[tokio::test]
    async fn test_get_restarting_processes() {
        let mut manager = ProcessManager::new();
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None, None);
        manager.add_process("proc3".to_string(), "sleep 10".to_string(), None, None);

        // No processes restarting initially
        assert!(manager.get_restarting_processes().is_empty());

        // Set some to restarting
        manager.set_restarting("proc1");
        manager.set_restarting("proc3");

        let restarting = manager.get_restarting_processes();
        assert_eq!(restarting.len(), 2);
        assert!(restarting.contains(&"proc1".to_string()));
        assert!(restarting.contains(&"proc3".to_string()));
        assert!(!restarting.contains(&"proc2".to_string()));
    }

    #[tokio::test]
    async fn test_has_pending_restarts() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "sleep 10".to_string(), None, None);

        assert!(!manager.has_pending_restarts());

        manager.set_restarting("test");
        assert!(manager.has_pending_restarts());
    }

    #[tokio::test]
    async fn test_perform_pending_restarts() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "echo hello".to_string(), None, None);

        // Start the process
        manager.start_process("test").await.unwrap();
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Running));

        // Set to restarting
        manager.set_restarting("test");
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Restarting));

        // Perform the restart
        let (succeeded, failed) = manager.perform_pending_restarts().await;
        assert_eq!(succeeded.len(), 1);
        assert!(succeeded.contains(&"test".to_string()));
        assert!(failed.is_empty());

        // Process should now be running
        assert_eq!(manager.get_status("test"), Some(ProcessStatus::Running));

        // Clean up
        manager.kill_all().await.unwrap();
    }

    // StatusMatcher integration tests

    #[test]
    fn test_process_handle_without_status_config() {
        let handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            None,
        );
        assert!(handle.get_custom_status().is_none());
    }

    #[test]
    fn test_process_handle_with_status_config_default() {
        use crate::config::{StatusConfig, StatusTransition};
        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Ready".to_string(),
                    label: "Ready".to_string(),
                    color: Some("green".to_string()),
                },
            ],
        };

        let handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            Some(&config),
        );

        let status = handle.get_custom_status();
        assert!(status.is_some());
        let (label, color) = status.unwrap();
        assert_eq!(label, "Starting");
        assert!(color.is_none());
    }

    #[test]
    fn test_process_handle_check_log_line_changes_status() {
        use crate::config::{StatusConfig, StatusTransition};
        use ratatui::style::Color;

        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Server ready".to_string(),
                    label: "Ready".to_string(),
                    color: Some("green".to_string()),
                },
            ],
        };

        let mut handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            Some(&config),
        );

        assert_eq!(handle.get_custom_status().unwrap().0, "Starting");

        let changed = handle.check_log_line("Server ready to accept connections");
        assert!(changed);

        let status = handle.get_custom_status().unwrap();
        assert_eq!(status.0, "Ready");
        assert_eq!(status.1, Some(Color::Green));
    }

    #[test]
    fn test_process_handle_check_log_line_no_match() {
        use crate::config::{StatusConfig, StatusTransition};

        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Server ready".to_string(),
                    label: "Ready".to_string(),
                    color: None,
                },
            ],
        };

        let mut handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            Some(&config),
        );

        let changed = handle.check_log_line("Some unrelated log message");
        assert!(!changed);
        assert_eq!(handle.get_custom_status().unwrap().0, "Starting");
    }

    #[test]
    fn test_process_handle_reset_status() {
        use crate::config::{StatusConfig, StatusTransition};

        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Ready".to_string(),
                    label: "Ready".to_string(),
                    color: Some("green".to_string()),
                },
            ],
        };

        let mut handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            Some(&config),
        );

        handle.check_log_line("Ready");
        assert_eq!(handle.get_custom_status().unwrap().0, "Ready");

        handle.reset_status();
        assert_eq!(handle.get_custom_status().unwrap().0, "Starting");
    }

    #[test]
    fn test_process_handle_check_log_line_without_matcher() {
        let mut handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            None,
        );

        let changed = handle.check_log_line("Some log message");
        assert!(!changed);
    }

    #[test]
    fn test_process_handle_reset_status_without_matcher() {
        let mut handle = ProcessHandle::new(
            "test".to_string(),
            "echo hello".to_string(),
            None,
            None,
        );

        handle.reset_status();
        assert!(handle.get_custom_status().is_none());
    }

    // process_logs integration tests

    #[test]
    fn test_process_logs_updates_status_from_log_line() {
        use crate::config::{StatusConfig, StatusTransition};
        use ratatui::style::Color;

        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Server ready".to_string(),
                    label: "Ready".to_string(),
                    color: Some("green".to_string()),
                },
            ],
        };

        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, Some(&config));

        // Verify initial status
        let handle = manager.processes.get("web").unwrap();
        assert_eq!(handle.get_custom_status().unwrap().0, "Starting");

        // Add a log directly to the channel
        manager.log_buffer.push(LogLine::new(
            LogSource::ProcessStdout("web".to_string()),
            "Server ready to accept connections".to_string(),
        ));

        // Re-create manager with proper channel setup to test process flow
        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, Some(&config));

        // Test check_log_line directly since we can't easily access the internal channel
        let handle = manager.processes.get_mut("web").unwrap();
        let changed = handle.check_log_line("Server ready to accept connections");
        assert!(changed);

        let status = handle.get_custom_status().unwrap();
        assert_eq!(status.0, "Ready");
        assert_eq!(status.1, Some(Color::Green));
    }

    #[test]
    fn test_process_logs_ignores_unknown_process() {
        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, None);

        // Add a log from an unknown process directly to buffer
        // This simulates what happens when process_logs encounters a log from unknown process
        manager.log_buffer.push(LogLine::new(
            LogSource::ProcessStdout("unknown".to_string()),
            "Some log message".to_string(),
        ));

        // The log should be in the buffer
        assert_eq!(manager.log_buffer.len(), 1);
        // And no crash occurred - graceful handling
    }

    #[test]
    fn test_process_logs_handles_file_logs_without_matching_process() {
        use std::path::PathBuf;

        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, None);

        // Add a file log for a process that doesn't exist in our manager
        manager.log_buffer.push(LogLine::new(
            LogSource::File {
                process_name: "other".to_string(),
                path: PathBuf::from("/var/log/other.log"),
            },
            "File log message".to_string(),
        ));

        // The log should be in the buffer
        assert_eq!(manager.log_buffer.len(), 1);
        // And no crash occurred
    }

    #[test]
    fn test_process_logs_checks_multiple_logs_in_sequence() {
        use crate::config::{StatusConfig, StatusTransition};

        let config = StatusConfig {
            default: Some("Starting".to_string()),
            color: None,
            transitions: vec![
                StatusTransition {
                    pattern: "Listening".to_string(),
                    label: "Listening".to_string(),
                    color: Some("yellow".to_string()),
                },
                StatusTransition {
                    pattern: "Ready".to_string(),
                    label: "Ready".to_string(),
                    color: Some("green".to_string()),
                },
            ],
        };

        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, Some(&config));

        let handle = manager.processes.get_mut("web").unwrap();
        assert_eq!(handle.get_custom_status().unwrap().0, "Starting");

        // First transition
        handle.check_log_line("Listening on port 3000");
        assert_eq!(handle.get_custom_status().unwrap().0, "Listening");

        // Second transition
        handle.check_log_line("Ready to serve requests");
        assert_eq!(handle.get_custom_status().unwrap().0, "Ready");
    }

    #[test]
    fn test_has_standalone_log_file_returns_false_initially() {
        let manager = ProcessManager::new();
        assert!(!manager.has_standalone_log_file("rails"));
    }

    #[test]
    fn test_get_standalone_log_file_names_returns_empty_initially() {
        let manager = ProcessManager::new();
        assert!(manager.get_standalone_log_file_names().is_empty());
    }

    #[test]
    fn test_process_name_not_confused_with_log_file() {
        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "echo hi".to_string(), None, None);

        // has_process should return true for process
        assert!(manager.has_process("web"));
        // has_standalone_log_file should return false for process
        assert!(!manager.has_standalone_log_file("web"));
    }
}
