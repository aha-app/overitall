# Example Setup

This directory contains example files for testing overitall.

## Files

- `Procfile` - Defines three processes: web, worker, and monitor
- `overitall.toml` - Configuration file with process-log mappings and example filters
- `web_server.rb` - Simulates a web server generating HTTP request logs
  - Outputs short HTTP request logs
  - Occasionally outputs long SQL queries (to test line truncation)
  - Occasionally outputs long JSON API responses (to test line truncation)
- `worker.rb` - Simulates a background worker processing jobs
  - Outputs job processing messages
  - Occasionally outputs long stack traces (to test line truncation)
  - Occasionally outputs complex job data structures (to test line truncation)
- `monitor.rb` - Simulates a system monitor reporting resource usage

## Testing

The Ruby scripts will generate log output to stdout and some will also write to log files:
- `web.log` - Web server logs
- `worker.log` - Worker logs
- Monitor writes to stdout only

You can test the scripts individually:
```bash
ruby example/web_server.rb
```

Or use overitall (once implemented) to run all processes from the Procfile:
```bash
cargo run -- --config example/overitall.toml
```
