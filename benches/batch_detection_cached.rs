use chrono::{Duration, Local};
use overitall::log::{LogLine, LogSource};
use overitall::ui::{BatchCache, BatchCacheKey};
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
    println!("=== Batch Detection CACHED Benchmark ===\n");

    for log_count in [1000, 5000, 10000, 20000] {
        let logs = create_test_logs(log_count);
        let refs: Vec<&LogLine> = logs.iter().collect();

        let key = BatchCacheKey::from_context(&refs, 100, 0, String::new(), 0, false, false);

        let mut cache = BatchCache::new();

        // Simulate 60fps rendering for 1 second (60 frames)
        let frames = 60;
        let start = Instant::now();
        for _ in 0..frames {
            let _ = cache.get_or_compute(&refs, 100, key.clone());
        }
        let elapsed = start.elapsed();

        println!("{} logs:", log_count);
        println!("  Total time for {} frames: {:?}", frames, elapsed);
        println!("  Per frame: {:?}", elapsed / frames);
        println!("  Cache hits: {}, misses: {}", cache.hits, cache.misses);
        println!(
            "  Hit rate: {:.1}%",
            (cache.hits as f64 / (cache.hits + cache.misses) as f64) * 100.0
        );
        println!();
    }

    println!("Compare with baseline to see speedup!");
}
