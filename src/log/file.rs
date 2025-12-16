use super::{LogLine, LogSource};
use anyhow::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File as StdFile;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

/// Reader for tailing log files using OS-level file system notifications
pub struct FileReader {
    process_name: String,
    path: PathBuf,
    task: Option<JoinHandle<()>>,
}

impl FileReader {
    pub fn new(process_name: String, path: PathBuf) -> Self {
        Self {
            process_name,
            path,
            task: None,
        }
    }

    /// Start tailing the log file and send lines to the channel
    pub async fn start(&mut self, log_tx: mpsc::UnboundedSender<LogLine>) -> Result<()> {
        if self.task.is_some() {
            return Ok(()); // Already running
        }

        let process_name = self.process_name.clone();
        let path = self.path.clone();

        let task = tokio::spawn(async move {
            tail_file_with_notify(path, process_name, log_tx).await;
        });

        self.task = Some(task);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Drop for FileReader {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Tail the file using notify for file system events
async fn tail_file_with_notify(
    path: PathBuf,
    process_name: String,
    log_tx: mpsc::UnboundedSender<LogLine>,
) {
    let mut position: Option<u64> = None;
    let mut line_buffer = String::new();

    // Create a channel for file change notifications
    let (notify_tx, mut notify_rx) = mpsc::unbounded_channel::<()>();

    // Determine what to watch - the file itself if it exists, otherwise the parent directory
    let watch_target = if path.exists() {
        path.clone()
    } else {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path.clone())
    };

    let target_path = path.clone();
    let tx = notify_tx.clone();

    // Create the watcher
    let _watcher = match RecommendedWatcher::new(
        move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let paths_match = event.paths.iter().any(|p| p == &target_path);
                let is_relevant = matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                );

                if is_relevant && (paths_match || event.paths.is_empty()) {
                    let _ = tx.send(());
                }
            }
        },
        Config::default(),
    ) {
        Ok(mut w) => {
            if w.watch(&watch_target, RecursiveMode::NonRecursive).is_err() {
                // Fall back to polling if watch fails
            }
            Some(w)
        }
        Err(_) => None,
    };

    loop {
        // Wait for file to exist
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Open the file
        let mut file = match StdFile::open(&path) {
            Ok(f) => f,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Get current file size
        let file_len = match file.metadata() {
            Ok(m) => m.len(),
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Initialize position to end of file on first run (tail -f behavior)
        let pos = position.get_or_insert(file_len);

        // If file was truncated, reset position to start
        if file_len < *pos {
            *pos = 0;
        }

        // Read any new content
        if file_len > *pos {
            if file.seek(SeekFrom::Start(*pos)).is_ok() {
                let mut reader = BufReader::new(file);

                loop {
                    line_buffer.clear();
                    match reader.read_line(&mut line_buffer) {
                        Ok(0) => break, // EOF
                        Ok(bytes_read) => {
                            *pos += bytes_read as u64;

                            let line = line_buffer.trim_end_matches('\n').trim_end_matches('\r');

                            if line.is_empty() && bytes_read <= 2 {
                                continue;
                            }

                            let log_line = LogLine::new(
                                LogSource::File {
                                    process_name: process_name.clone(),
                                    path: path.clone(),
                                },
                                line.to_string(),
                            );

                            if log_tx.send(log_line).is_err() {
                                return; // Channel closed
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        // Wait for next file change notification (with timeout as fallback)
        tokio::select! {
            _ = notify_rx.recv() => {
                // Got notification, loop will read new content
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                // Timeout - check file anyway (fallback for missed notifications)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_reader_skips_existing_content() {
        // Create a temporary file with existing content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "old line 1").unwrap();
        writeln!(temp_file, "old line 2").unwrap();
        writeln!(temp_file, "old line 3").unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_path_buf();

        let (log_tx, mut log_rx) = mpsc::unbounded_channel();
        let mut reader = FileReader::new("test".to_string(), path.clone());

        reader.start(log_tx).await.unwrap();

        // Give it time to start and process
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should NOT receive old lines (position initialized to end of file)
        let mut logs = Vec::new();
        while let Ok(log) = log_rx.try_recv() {
            logs.push(log);
        }

        assert!(
            logs.is_empty(),
            "Should not read existing content, but got {} lines: {:?}",
            logs.len(),
            logs.iter().map(|l| &l.line).collect::<Vec<_>>()
        );

        reader.stop();
    }

    #[tokio::test]
    async fn test_file_reader_picks_up_new_content() {
        // Create a temporary file with existing content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "old line").unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_path_buf();

        let (log_tx, mut log_rx) = mpsc::unbounded_channel();
        let mut reader = FileReader::new("test".to_string(), path.clone());

        reader.start(log_tx).await.unwrap();

        // Give it time to start and initialize position at end of file
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Now write NEW content after the reader has started
        {
            use std::fs::OpenOptions;
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            writeln!(file, "new line 1").unwrap();
            writeln!(file, "new line 2").unwrap();
            file.flush().unwrap();
            file.sync_all().unwrap();
        }

        // Wait for the reader to pick up the new content
        let mut logs = Vec::new();
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            while let Ok(log) = log_rx.try_recv() {
                logs.push(log);
            }
            if logs.len() >= 2 {
                break;
            }
        }

        assert_eq!(
            logs.len(),
            2,
            "Should receive 2 new lines, got {}: {:?}",
            logs.len(),
            logs.iter().map(|l| &l.line).collect::<Vec<_>>()
        );
        assert_eq!(logs[0].line, "new line 1");
        assert_eq!(logs[1].line, "new line 2");

        reader.stop();
    }
}
