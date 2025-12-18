# Example Setup

This directory contains example files for testing overitall.

## Files

- `Procfile` - Defines three processes: web, worker, and monitor
- `overitall.toml` - Configuration file with process-log mappings and example filters
- `web_server.rb` - Simulates a web server generating HTTP request logs at high volume
  - Generates logs every 50-200ms (5-20 logs/second)
  - Outputs short HTTP request logs with color-coded status codes
  - Occasionally generates bursts of 3-8 logs together (to test batch detection)
  - Occasionally outputs long SQL queries (to test line truncation)
  - Occasionally outputs long JSON API responses (to test line truncation)
- `worker.rb` - Simulates a background worker processing jobs at high volume
  - Generates logs every 100-500ms (2-10 logs/second)
  - Outputs job processing messages with completion/failure status
  - Occasionally outputs long stack traces (to test line truncation)
  - Occasionally outputs complex job data structures (to test line truncation)
- `monitor.rb` - Simulates a system monitor reporting resource usage
  - Generates logs every 500-1500ms (0.6-2 logs/second)
  - Outputs CPU, memory, disk, and network metrics with color coding
  - Occasionally checks service health status

## Testing

The Ruby scripts generate logs at high volume to test performance and batch detection:
- **Combined**: ~10-30 logs/second from all three processes
- **Web server**: 5-20 logs/second (with occasional bursts)
- **Worker**: 2-10 logs/second
- **Monitor**: 0.6-2 logs/second

Logs are written to:
- `web.log` - Web server logs
- `worker.log` - Worker logs
- Monitor writes to stdout only

You can test the scripts individually:
```bash
ruby example/web_server.rb
```

Or use overitall to run all processes from the Procfile:
```bash
cargo run -- --config example/overitall.toml
```

The high-volume log generation is perfect for testing:
- Buffer eviction under memory pressure
- Batch detection with varying time windows
- Scrolling performance with large log volumes
- Line selection and navigation at scale
- Filter performance with thousands of logs

## Custom Process Status Labels

The example config demonstrates the custom process status feature. Each process has a custom status configuration that shows meaningful labels based on log patterns:

- **web**: Shows "Starting" initially, then "Ready" (green) when HTTP requests start
- **worker**: Shows "Starting" initially, then "Processing" (cyan) when jobs begin
- **monitor**: Shows "Starting" initially, then "Active" (blue) when metrics start flowing

Watch the process list at the top of the TUI - you'll see the status labels change from "Starting" to their active state within the first second as logs come in.
