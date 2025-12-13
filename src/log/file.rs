use super::{LogLine, LogSource};
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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
            // Try to open the file, if it doesn't exist yet, wait for it
            let mut file = loop {
                match File::open(&path).await {
                    Ok(f) => break f,
                    Err(_) => {
                        // File doesn't exist yet, wait a bit and retry
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            };

            // Seek to end of file to only read new content (tail behavior)
            let _ = file.seek(std::io::SeekFrom::End(0)).await;

            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let log_line = LogLine::new(
                    LogSource::File {
                        process_name: process_name.clone(),
                        path: path.clone(),
                    },
                    line,
                );

                if log_tx.send(log_line).is_err() {
                    // Channel closed, stop reading
                    break;
                }
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
}
