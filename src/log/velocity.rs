use chrono::{DateTime, Duration, Local};
use std::collections::VecDeque;

const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Tracks log arrival counts in time buckets for sparkline display
pub struct LogVelocityTracker {
    /// Counts per bucket (most recent at back)
    buckets: VecDeque<u32>,
    /// Number of buckets to maintain
    num_buckets: usize,
    /// Duration of each bucket in seconds
    bucket_seconds: i64,
    /// Start time of the current (most recent) bucket
    current_bucket_start: DateTime<Local>,
}

impl LogVelocityTracker {
    /// Create tracker with specified buckets and bucket duration
    /// Default: 12 buckets of 5 seconds each = 60 seconds of history
    pub fn new(num_buckets: usize, bucket_seconds: i64) -> Self {
        let mut buckets = VecDeque::with_capacity(num_buckets);
        buckets.push_back(0);
        Self {
            buckets,
            num_buckets,
            bucket_seconds,
            current_bucket_start: Local::now(),
        }
    }

    /// Create with default settings (12 buckets, 5 seconds each = 60 seconds)
    pub fn default() -> Self {
        Self::new(12, 5)
    }

    /// Record a log arrival at the given time
    pub fn record(&mut self, arrival_time: DateTime<Local>) {
        let bucket_duration = Duration::seconds(self.bucket_seconds);
        let bucket_end = self.current_bucket_start + bucket_duration;

        if arrival_time >= self.current_bucket_start && arrival_time < bucket_end {
            // Still in current bucket, increment
            if let Some(count) = self.buckets.back_mut() {
                *count = count.saturating_add(1);
            }
        } else if arrival_time >= bucket_end {
            // Time has advanced past current bucket
            let time_diff = arrival_time - self.current_bucket_start;
            let buckets_to_advance =
                (time_diff.num_seconds() / self.bucket_seconds).max(1) as usize;

            // Push empty buckets for any skipped periods (minus 1 for the new bucket)
            for _ in 1..buckets_to_advance {
                self.buckets.push_back(0);
            }

            // Start new bucket with count=1
            self.buckets.push_back(1);

            // Update bucket start time
            self.current_bucket_start =
                self.current_bucket_start + Duration::seconds(self.bucket_seconds * buckets_to_advance as i64);

            // Trim front if exceeds num_buckets
            while self.buckets.len() > self.num_buckets {
                self.buckets.pop_front();
            }
        }
        // If arrival_time < current_bucket_start (past log), we ignore it
    }

    /// Get current bucket counts for rendering
    #[allow(dead_code)]
    pub fn get_buckets(&self) -> &VecDeque<u32> {
        &self.buckets
    }

    /// Generate sparkline string from current buckets
    pub fn sparkline(&self) -> String {
        if self.buckets.is_empty() {
            return String::new();
        }

        let max = *self.buckets.iter().max().unwrap_or(&1).max(&1);

        self.buckets
            .iter()
            .map(|&count| {
                if count == 0 {
                    ' '
                } else {
                    let idx = ((count as f64 / max as f64) * 7.0).round() as usize;
                    BLOCKS[idx.min(7)]
                }
            })
            .collect()
    }

    /// Generate sparkline with minimum bar for zero (maintains visual line)
    #[allow(dead_code)]
    pub fn sparkline_with_baseline(&self) -> String {
        if self.buckets.is_empty() {
            return String::new();
        }

        let max = *self.buckets.iter().max().unwrap_or(&1).max(&1);

        self.buckets
            .iter()
            .map(|&count| {
                if count == 0 {
                    '▁'
                } else {
                    let idx = ((count as f64 / max as f64) * 7.0).round() as usize;
                    BLOCKS[idx.min(7)]
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_has_single_empty_bucket() {
        let tracker = LogVelocityTracker::new(12, 5);
        assert_eq!(tracker.buckets.len(), 1);
        assert_eq!(tracker.buckets[0], 0);
    }

    #[test]
    fn test_basic_recording_increments_bucket() {
        let mut tracker = LogVelocityTracker::new(12, 5);
        let start = tracker.current_bucket_start;

        // Record several logs at same bucket time
        tracker.record(start);
        tracker.record(start + Duration::seconds(1));
        tracker.record(start + Duration::seconds(2));

        assert_eq!(tracker.buckets.len(), 1);
        assert_eq!(tracker.buckets[0], 3);
    }

    #[test]
    fn test_bucket_advancement() {
        let mut tracker = LogVelocityTracker::new(12, 5);
        let start = tracker.current_bucket_start;

        // Record in first bucket
        tracker.record(start);
        tracker.record(start + Duration::seconds(1));

        // Record in second bucket (5+ seconds later)
        tracker.record(start + Duration::seconds(6));

        assert_eq!(tracker.buckets.len(), 2);
        assert_eq!(tracker.buckets[0], 2); // First bucket had 2 logs
        assert_eq!(tracker.buckets[1], 1); // Second bucket has 1 log
    }

    #[test]
    fn test_time_gaps_fill_with_zeros() {
        let mut tracker = LogVelocityTracker::new(12, 5);
        let start = tracker.current_bucket_start;

        // Record in first bucket
        tracker.record(start);

        // Skip 2 bucket periods (10+ seconds later)
        tracker.record(start + Duration::seconds(16));

        assert_eq!(tracker.buckets.len(), 4);
        assert_eq!(tracker.buckets[0], 1); // First bucket
        assert_eq!(tracker.buckets[1], 0); // Skipped
        assert_eq!(tracker.buckets[2], 0); // Skipped
        assert_eq!(tracker.buckets[3], 1); // New bucket
    }

    #[test]
    fn test_bucket_trimming() {
        let mut tracker = LogVelocityTracker::new(4, 5);
        let start = tracker.current_bucket_start;

        // Fill up 6 buckets worth of time
        for i in 0..6 {
            tracker.record(start + Duration::seconds(i * 5));
        }

        // Should only keep last 4 buckets
        assert_eq!(tracker.buckets.len(), 4);
    }

    #[test]
    fn test_sparkline_empty() {
        let mut tracker = LogVelocityTracker::new(12, 5);
        tracker.buckets.clear();
        assert_eq!(tracker.sparkline(), "");
    }

    #[test]
    fn test_sparkline_all_zeros() {
        let mut tracker = LogVelocityTracker::new(4, 5);
        tracker.buckets = VecDeque::from([0, 0, 0, 0]);
        assert_eq!(tracker.sparkline(), "    "); // All spaces
    }

    #[test]
    fn test_sparkline_rendering() {
        let mut tracker = LogVelocityTracker::new(8, 5);
        tracker.buckets = VecDeque::from([1, 2, 4, 8, 4, 2, 1, 0]);

        let sparkline = tracker.sparkline();
        assert_eq!(sparkline.chars().count(), 8);
        // Max is 8, so:
        // 1 -> 0.125*7 = 0.875 -> round to 1 -> ▂
        // 2 -> 0.25*7 = 1.75 -> round to 2 -> ▃
        // 4 -> 0.5*7 = 3.5 -> round to 4 -> ▅
        // 8 -> 1.0*7 = 7 -> █
        // 0 -> space
        assert_eq!(sparkline, "▂▃▅█▅▃▂ ");
    }

    #[test]
    fn test_sparkline_single_value() {
        let mut tracker = LogVelocityTracker::new(4, 5);
        tracker.buckets = VecDeque::from([0, 0, 5, 0]);
        let sparkline = tracker.sparkline();
        assert_eq!(sparkline, "  █ "); // Only one bar at max
    }

    #[test]
    fn test_sparkline_with_baseline() {
        let mut tracker = LogVelocityTracker::new(4, 5);
        tracker.buckets = VecDeque::from([0, 5, 0, 10]);
        let sparkline = tracker.sparkline_with_baseline();
        // 5/10 * 7 = 3.5 -> round to 4 -> ▅
        assert_eq!(sparkline, "▁▅▁█");
    }

    #[test]
    fn test_past_logs_ignored() {
        let mut tracker = LogVelocityTracker::new(12, 5);
        let start = tracker.current_bucket_start;

        tracker.record(start);
        let initial_count = tracker.buckets[0];

        // Try to record a log from the past
        tracker.record(start - Duration::seconds(10));

        // Count should not change
        assert_eq!(tracker.buckets[0], initial_count);
    }

    #[test]
    fn test_saturating_add() {
        let mut tracker = LogVelocityTracker::new(1, 5);
        tracker.buckets[0] = u32::MAX - 1;

        let start = tracker.current_bucket_start;
        tracker.record(start);
        tracker.record(start);
        tracker.record(start);

        // Should saturate at MAX, not overflow
        assert_eq!(tracker.buckets[0], u32::MAX);
    }

    #[test]
    fn test_default_constructor() {
        let tracker = LogVelocityTracker::default();
        assert_eq!(tracker.num_buckets, 12);
        assert_eq!(tracker.bucket_seconds, 5);
    }
}
