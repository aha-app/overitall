# Overitall - Project Plan

## Overview
Rust TUI combining overmind (process management) + lnav (log viewing).

See [README.md](README.md) for features and usage.
See [todo.md](todo.md) for current priorities.
See [ARCHITECTURE.md](ARCHITECTURE.md) for code structure.

## Configuration
- Config file: `.overitall.toml` (override with `--config` or `-c`)
- Procfile path + process-to-logfile mapping
- Filters auto-saved to config

## Build & Test
```bash
cargo build              # Build
cargo test               # Run tests
cargo insta review       # Review snapshot changes
cargo run -- -c example/overitall.toml  # Run with example config
```

## CPU Profiling

Profile oit with the example app (generates ~10-30 logs/second):

```bash
# Build release version
cargo build --release

# Open oit in a real Terminal window (TUI needs a terminal)
PROJ_DIR="$(pwd)"
osascript <<EOF
tell application "Terminal"
    activate
    do script "cd $PROJ_DIR && ./target/release/oit -c example/overitall.toml"
end tell
EOF

# Wait for logs to accumulate
sleep 15

# Find and sample the oit process
OIT_PID=$(pgrep -x oit | head -1)
sample $OIT_PID 5 -file /tmp/oit-sample.txt

# Kill oit when done
pkill -9 oit
pkill -f "ruby.*example"

# Analyze the sample
head -400 /tmp/oit-sample.txt                    # Call graph
grep -A 50 "Sort by top of stack" /tmp/oit-sample.txt  # Hotspots
```

## Planned Features
- JSON log pretty-print in expand view
- Timestamp-based navigation (`:goto 14:30`)
- Log export command (`:export logs.txt`)
