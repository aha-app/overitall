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
    pub working_dir: Option<PathBuf>,
    pub status: ProcessStatus,
    child: Option<Child>,
    pgid: Option<i32>,  // Process Group ID for killing entire process tree
    stdout_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
}

impl ProcessHandle {
    /// Create a new process handle (not yet started)
    pub fn new(name: String, command: String, working_dir: Option<PathBuf>) -> Self {
        Self {
            name,
            command,
            working_dir,
            status: ProcessStatus::Stopped,
            child: None,
            pgid: None,
            stdout_task: None,
            stderr_task: None,
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
    pub fn add_process(&mut self, name: String, command: String, working_dir: Option<PathBuf>) {
        self.processes.insert(name.clone(), ProcessHandle::new(name, command, working_dir));
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
        manager.add_process("test".to_string(), "echo hello".to_string(), None);

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
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None);

        manager.start_all().await.unwrap();
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
        manager.add_process("proc1".to_string(), "sleep 10".to_string(), None);
        manager.add_process("proc2".to_string(), "sleep 10".to_string(), None);
        manager.add_process("proc3".to_string(), "sleep 10".to_string(), None);

        manager.start_all().await.unwrap();

        manager.kill_all().await.unwrap();

        assert_eq!(manager.get_status("proc1"), Some(ProcessStatus::Stopped));
        assert_eq!(manager.get_status("proc2"), Some(ProcessStatus::Stopped));
        assert_eq!(manager.get_status("proc3"), Some(ProcessStatus::Stopped));
    }

    #[tokio::test]
    async fn test_shutdown_flow_sets_status_before_killing() {
        let mut manager = ProcessManager::new();
        manager.add_process("test".to_string(), "sleep 10".to_string(), None);

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
        manager.add_process("slow".to_string(), "sleep 10".to_string(), None);

        manager.start_all().await.unwrap();
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
}
