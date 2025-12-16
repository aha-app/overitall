use overitall::ui::ansi_cache::{AnsiCache, AnsiCacheKey};
use overitall::ui::utils::parse_ansi_to_spans;
use std::time::Instant;

/// Generate realistic log lines with ANSI color codes
fn generate_ansi_log_lines(count: usize) -> Vec<(u64, String)> {
    let mut lines = Vec::with_capacity(count);

    // ANSI escape codes for colors
    let red = "\x1b[31m";
    let green = "\x1b[32m";
    let yellow = "\x1b[33m";
    let blue = "\x1b[34m";
    let reset = "\x1b[0m";
    let bold = "\x1b[1m";

    for i in 0..count {
        let line = match i % 5 {
            0 => format!(
                "[12:34:56] {}web{}: {}INFO{} - {}Request processed{} in {}25ms{}",
                blue, reset, green, reset, bold, reset, yellow, reset
            ),
            1 => format!(
                "[12:34:56] {}worker{}: {}ERROR{} - {}Failed to process job{} #{}",
                blue, reset, red, reset, bold, reset, i
            ),
            2 => format!(
                "[12:34:56] {}api{}: {}DEBUG{} - Params: {}{{\"user_id\": {}{}{}, \"action\": \"create\" }}{}",
                blue, reset, yellow, reset, bold, green, i, reset, reset
            ),
            3 => format!(
                "[12:34:56] {}scheduler{}: {}WARN{} - {}Job queue depth:{} {}{}{} exceeds threshold",
                blue, reset, yellow, reset, bold, reset, red, i * 10, reset
            ),
            _ => format!(
                "[12:34:56] {}mailer{}: {}INFO{} - Sent email to {}user{}@example.com",
                blue, reset, green, reset, bold, reset
            ),
        };
        lines.push((i as u64, line));
    }

    lines
}

/// Benchmark parsing without cache (simulates pre-cache behavior)
fn bench_without_cache(lines: &[(u64, String)], frames: usize) -> std::time::Duration {
    let start = Instant::now();

    for _ in 0..frames {
        for (_id, line) in lines {
            // Parse ANSI codes every frame (old behavior)
            let _spans = parse_ansi_to_spans(line);
        }
    }

    start.elapsed()
}

/// Benchmark parsing with cache (current behavior)
fn bench_with_cache(lines: &[(u64, String)], frames: usize) -> std::time::Duration {
    let mut cache = AnsiCache::new(2000);
    let start = Instant::now();

    for _ in 0..frames {
        for (id, line) in lines {
            let key = AnsiCacheKey::new(*id, false);
            let _cached = cache.get_or_parse(key, line);
        }
    }

    start.elapsed()
}

fn main() {
    println!("=== ANSI Parsing Cache Benchmark ===\n");
    println!("This benchmark measures the performance improvement from caching");
    println!("parsed ANSI escape sequences instead of re-parsing every frame.\n");

    // Test different log counts
    for log_count in [100, 500, 1000, 2000] {
        let lines = generate_ansi_log_lines(log_count);

        // Simulate 60fps for 1 second (60 frames)
        let frames = 60;

        println!("--- {} log lines, {} frames (simulating 1 second at 60fps) ---", log_count, frames);

        // Warm up
        let _ = bench_without_cache(&lines, 1);
        let _ = bench_with_cache(&lines, 1);

        // Actual benchmark
        let uncached_time = bench_without_cache(&lines, frames);
        let cached_time = bench_with_cache(&lines, frames);

        let speedup = uncached_time.as_secs_f64() / cached_time.as_secs_f64();

        println!("  Without cache: {:?} ({:.2} ms/frame)",
            uncached_time,
            uncached_time.as_secs_f64() * 1000.0 / frames as f64);
        println!("  With cache:    {:?} ({:.2} ms/frame)",
            cached_time,
            cached_time.as_secs_f64() * 1000.0 / frames as f64);
        println!("  Speedup:       {:.1}x", speedup);

        // Calculate if we can maintain 60fps (16.67ms frame budget)
        let frame_budget_ms = 16.67;
        let uncached_ms_per_frame = uncached_time.as_secs_f64() * 1000.0 / frames as f64;
        let cached_ms_per_frame = cached_time.as_secs_f64() * 1000.0 / frames as f64;

        println!("  Frame budget:  {:.2}ms (for 60fps)", frame_budget_ms);
        println!("  Uncached uses: {:.1}% of budget", uncached_ms_per_frame / frame_budget_ms * 100.0);
        println!("  Cached uses:   {:.1}% of budget", cached_ms_per_frame / frame_budget_ms * 100.0);
        println!();
    }

    // Test cache hit rate over multiple frames
    println!("=== Cache Hit Rate Test ===\n");
    let lines = generate_ansi_log_lines(1000);
    let mut cache = AnsiCache::new(2000);

    // First frame - all misses
    for (id, line) in &lines {
        let key = AnsiCacheKey::new(*id, false);
        let _ = cache.get_or_parse(key, line);
    }

    // Next 59 frames - should be all hits
    for _ in 0..59 {
        for (id, line) in &lines {
            let key = AnsiCacheKey::new(*id, false);
            let _ = cache.get_or_parse(key, line);
        }
    }

    println!("After 60 frames with 1000 log lines:");
    println!("  Total lookups: 60,000");
    println!("  Expected hits: 59,000 (59 frames after initial parse)");
    println!("  Expected hit rate: 98.3%");
    println!();

    println!("=== Summary ===\n");
    println!("The ANSI parsing cache provides significant speedup by avoiding");
    println!("repeated parsing of the same log lines across frames. The cache");
    println!("is most effective when viewing a stable set of logs (typical case).");
}
