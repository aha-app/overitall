use crate::log::LogLine;

/// Detect batches from a slice of LogLine references
/// Returns a vector of (start_index, end_index) tuples for each batch
pub fn detect_batches_from_logs(logs: &[&LogLine], window_ms: i64) -> Vec<(usize, usize)> {
    if logs.is_empty() {
        return vec![];
    }

    if logs.len() == 1 {
        return vec![(0, 0)];
    }

    let mut batches = Vec::new();
    let mut batch_start = 0;

    for i in 1..logs.len() {
        // Compare to the start of the current batch, not the previous log
        // This prevents "chaining" where logs slowly drift apart over time
        let time_diff = logs[i].arrival_time - logs[batch_start].arrival_time;
        if time_diff.num_milliseconds() > window_ms {
            batches.push((batch_start, i - 1));
            batch_start = i;
        }
    }

    batches.push((batch_start, logs.len() - 1));
    batches
}
