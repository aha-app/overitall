use super::{LogLine, LogSource};
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;

/// Reader for tailing log files
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
            // Track our position in the file (None = not initialized yet)
            let mut position: Option<u64> = None;
            let mut line_buffer = String::new();

            loop {
                // Open file (or wait for it to exist)
                let file = loop {
                    match File::open(&path).await {
                        Ok(f) => break f,
                        Err(_) => {
                            // File doesn't exist yet, wait a bit and retry
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }
                };

                // Get current file size
                let metadata = match file.metadata().await {
                    Ok(m) => m,
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let file_len = metadata.len();

                // Initialize position to end of file on first run (tail -f behavior)
                let pos = position.get_or_insert(file_len);

                // If file was truncated, reset position to start
                if file_len < *pos {
                    *pos = 0;
                }

                // If no new content, wait and retry
                if file_len <= *pos {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                // Seek to our position and read new content
                let mut file = file;
                if let Err(_) = file.seek(std::io::SeekFrom::Start(*pos)).await {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                let mut reader = BufReader::new(file);

                // Read all available lines
                loop {
                    line_buffer.clear();
                    match reader.read_line(&mut line_buffer).await {
                        Ok(0) => {
                            // EOF reached, break to outer loop
                            break;
                        }
                        Ok(bytes_read) => {
                            *pos += bytes_read as u64;

                            // Remove trailing newline
                            let line = line_buffer.trim_end_matches('\n').trim_end_matches('\r');

                            // Skip empty lines that are just newlines
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
                                // Channel closed, stop reading entirely
                                return;
                            }
                        }
                        Err(_) => {
                            // Read error, break to outer loop to retry
                            break;
                        }
                    }
                }

                // Small delay before checking for more content
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
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
        // If it were reading from the beginning, it would read the old lines
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Should NOT receive old lines (file was seeked to end before reading)
        let mut logs = Vec::new();
        while let Ok(log) = log_rx.try_recv() {
            logs.push(log);
        }

        // The key test: no old content should be read
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
        tokio::time::sleep(Duration::from_millis(150)).await;

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
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            while let Ok(log) = log_rx.try_recv() {
                logs.push(log);
            }
            if logs.len() >= 2 {
                break;
            }
        }

        // Should have received the new lines
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
