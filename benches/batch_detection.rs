use chrono::{Duration, Local};
use overitall::log::{LogLine, LogSource};
use overitall::ui::detect_batches_from_logs;
use std::time::Instant;

fn create_test_logs(count: usize) -> Vec<LogLine> {
    let base_time = Local::now();
    (0..count)
        .map(|i| {
            let arrival = base_time + Duration::milliseconds((i as i64) * 10);
            LogLine {
                id: i as u64,
                line: format!("Log line {}", i),
                timestamp: arrival,
                arrival_time: arrival,
                source: LogSource::ProcessStdout("test".to_string()),
            }
        })
        .collect()
}

fn main() {
    println!("=== Batch Detection Baseline Benchmark ===\n");

    for log_count in [1000, 5000, 10000, 20000] {
        let logs = create_test_logs(log_count);
        let refs: Vec<&LogLine> = logs.iter().collect();

        // Simulate 60fps rendering for 1 second (60 frames)
        let frames = 60;
        let start = Instant::now();
        for _ in 0..frames {
            let _ = detect_batches_from_logs(&refs, 100);
        }
        let elapsed = start.elapsed();

        println!("{} logs:", log_count);
        println!("  Total time for {} frames: {:?}", frames, elapsed);
        println!("  Per frame: {:?}", elapsed / frames);
        println!(
            "  Ops/sec (if 60fps): {:.0}",
            60.0 / (elapsed.as_secs_f64() / frames as f64)
        );
        println!();
    }

    println!("Save these numbers for comparison with cached implementation!");
}
