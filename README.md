# Overitall (`oit`)

A terminal user interface (TUI) for process and log management, combining the best of [overmind](https://github.com/DarthSim/overmind) (process management) and [lnav](https://lnav.org/) (log viewing).

## What is Overitall?

Overitall (`oit`) is a Rust-based TUI that helps you manage multiple processes and their logs in a single, interactive interface. It's perfect for development environments where you need to run and monitor multiple services simultaneously (like Rails apps with web servers, background workers, and other services).

### Key Features

- **Process Management**: Start, stop, and restart processes defined in a Procfile
- **Unified Log Viewing**: View logs from multiple sources in a single, interleaved stream
- **Advanced Filtering**: Include or exclude log lines with regex patterns
- **Search**: Full-text search with highlighting across all logs
- **Batch Navigation**: Navigate through groups of related log lines that arrived together
- **Persistent Configuration**: Filters and settings are automatically saved
- **Vim-style Commands**: Familiar `:command` interface for power users

## Installation

### From Source

```bash
git clone https://github.com/yourusername/overitall.git
cd overitall
cargo build --release
```

The binary will be at `target/release/oit`. You can copy it to your PATH:

```bash
cp target/release/oit /usr/local/bin/
```

## Quick Start

1. Create a `Procfile` in your project directory:

```procfile
web: bundle exec rails server
worker: bundle exec sidekiq
```

2. Generate a default configuration file:

```bash
oit --init
```

This creates a `.overitall.toml` file with all processes from your Procfile:

```toml
procfile = "Procfile"
batch_window_ms = 100

[processes.web]
log_file = "logs/web.log"

[processes.worker]
log_file = "logs/worker.log"

[filters]
include = []
exclude = []
```

Edit the generated config to customize log file paths and other settings.

Alternatively, you can create the config file manually or specify a custom path:

```bash
oit --init -c custom-config.toml
```

3. Run `oit`:

```bash
oit
```

Or specify a custom config file:

```bash
oit --config path/to/config.toml
oit -c path/to/config.toml
```

## Usage

### Keyboard Shortcuts

#### Navigation
- `↑` / `↓` - Select previous/next log line
- `Enter` - Expand selected line (show full content in overlay)
- `j` / `k` - Scroll down/up
- `g` / `G` - Jump to top/bottom
- `?` - Show help overlay
- `q` - Quit application

#### Modes
- `:` - Enter command mode
- `/` - Enter search mode
- `Esc` - Exit current mode, close overlays, or jump to latest logs

#### Search
- `n` / `N` - Next/previous search match

#### Batch Navigation
- `[` / `]` - Previous/next batch

#### Clipboard & Batch Operations
- `c` - Copy selected line to clipboard (with timestamp and process)
- `C` - Copy entire batch to clipboard (all lines in batch)
- `b` - Focus on batch containing the selected line

### Commands

All commands are entered by pressing `:` followed by the command.

#### Process Management

- `:s <name>` - Start a process
- `:r <name>` - Restart a process
- `:k <name>` - Kill (stop) a process

Example:
```
:r worker    # Restart the worker process
:k web       # Stop the web process
:s web       # Start the web process
```

#### Filtering

- `:f <pattern>` - Add include filter (show only matching lines)
- `:fn <pattern>` - Add exclude filter (hide matching lines)
- `:fc` - Clear all filters
- `:fl` - List active filters

Filters support regex patterns:
```
:f ERROR                    # Show only lines containing ERROR
:fn DEBUG                   # Hide lines containing DEBUG
:f \[Worker\]              # Show only lines from Worker (escaped brackets)
```

#### Batch Navigation

Log lines that arrive within a short time window are grouped into "batches". This helps you see related log output together.

- `:nb` - Next batch
- `:pb` - Previous batch
- `:sb` - Toggle batch view mode (show only current batch)
- `:bw <milliseconds>` - Set batch window (default: 100ms)

You can also use `[` and `]` keys for quick batch navigation.

The batch window determines how close in time log lines must be to be grouped together. Adjust it based on your application's logging patterns (e.g., `:bw 1000` for 1 second window).

#### Search

- `/` - Enter search mode
- Type your search term and press Enter
- `n` - Jump to next match
- `N` - Jump to previous match
- `Esc` - Exit search mode

Search results are highlighted in the log view, with the current match shown in yellow.

## Configuration

The configuration file (`.overitall.toml` by default) uses TOML format:

```toml
# Path to your Procfile
procfile = "Procfile"

# Process-specific configuration
[processes.web]
log_file = "log/web.log"

[processes.worker]
log_file = "log/worker.log"

[processes.scheduler]
log_file = "log/scheduler.log"

# Filters (automatically saved when you add/remove filters)
[filters]
include = ["INFO", "ERROR"]
exclude = ["DEBUG"]
```

### Configuration Options

- `procfile` - Path to your Procfile (required)
- `processes.<name>.log_file` - Path to the log file for a specific process (optional)
- `filters.include` - Array of regex patterns to include
- `filters.exclude` - Array of regex patterns to exclude
- `max_log_buffer_mb` - Maximum memory for log buffer in megabytes (default: 50)
- `batch_window_ms` - Batch grouping window in milliseconds (default: 100)

### Memory Management

By default, Overitall limits the log buffer to 50 MB. When this limit is reached, the oldest logs are automatically evicted (First-In-First-Out).

Configure the buffer size in your `.overitall.toml`:

```toml
max_log_buffer_mb = 100  # Allow up to 100 MB of logs
```

The status bar shows current buffer usage and warns when eviction occurs. This prevents memory issues with long-running processes and high-volume logs.

## Example Setup

An example configuration is provided in the `example/` directory:

```bash
cd example
oit -c overitall.toml
```

This runs a simple demo with mock processes that generate log output for testing.

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run in development
cargo run

# Run with custom config
cargo run -- -c example/overitall.toml
```

### Testing

```bash
# Run all tests
cargo test

# Review snapshot test changes
cargo install cargo-insta
cargo insta review
```

## Architecture

Overitall is built with a modular architecture:

- **Log System**: Extensible log sources, parsing, buffering, and filtering
- **Process Manager**: Manages process lifecycle and delegates log handling to log module
- **Configuration**: TOML-based configuration with auto-save for filters
- **TUI**: Built with ratatui, testable with TestBackend for snapshot testing

### Key Dependencies

- **ratatui** - Terminal UI framework
- **tokio** - Async runtime for process management
- **crossterm** - Terminal manipulation
- **serde/toml** - Configuration parsing
- **chrono** - Timestamp tracking
- **regex** - Pattern matching for filters and search

## Project Status

Overitall is under active development. Current features:

- Easy initialization with `--init` flag (automatically generates config from Procfile)
- Process management (start/stop/restart)
- Log file tailing and interleaved viewing
- Filtering (include/exclude patterns)
- Search with highlighting
- Batch detection and navigation
- Line selection and expanded view (view full content of long log lines)
- Clipboard operations (copy lines and batches to system clipboard)
- Batch focus from selected line
- Dynamic batch window configuration (adjust batch grouping on-the-fly)
- Persistent configuration
- Help system

Planned features:

- Rails-specific log format parsing
- Additional log sources (syslog, HTTP endpoints)
- Performance optimizations for large log volumes
- Extended metadata display

## License

[Your license here]

## Contributing

[Your contributing guidelines here]
