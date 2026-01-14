# Overitall (`oit`)

A terminal user interface (TUI) for process and log management, combining the best of [overmind](https://github.com/DarthSim/overmind) (process management) and [lnav](https://lnav.org/) (log viewing).

## What is Overitall?

Overitall (`oit`) is a TUI that helps you manage multiple processes and their logs in a single, interactive interface. It's perfect for development environments where you need to run and monitor multiple services simultaneously (like Rails apps with web servers, background workers, and other services).

### Key Features

- **Process Management**: Start, stop, and restart processes defined in a Procfile
- **Custom Status Labels**: Show meaningful status like "Starting", "Ready" based on log patterns
- **Standalone Log Files**: Tail log files (like Rails logs) without an associated process
- **Unified Log Viewing**: View logs from multiple sources in a single, interleaved stream
- **Advanced Filtering**: Include or exclude log lines with regex patterns
- **Process Visibility Toggle**: Hide/show logs from specific processes on demand
- **Search**: Full-text search with highlighting across all logs
- **Batch Navigation**: Navigate through groups of related log lines that arrived together
- **Time Navigation**: Jump to specific timestamps with `:goto HH:MM` or relative times like `:goto -5m`
- **Trace Detection**: Find correlation IDs (UUIDs, trace IDs) and filter to specific traces
- **Persistent Configuration**: Filters and settings are automatically saved
- **Compact Mode**: Collapse verbose metadata tags (`[key:value]`) into `[+N]` for cleaner log viewing
- **Process Coloring**: Optional distinct colors per process for easier visual identification
- **Auto-Update**: Automatically checks for and installs updates on startup
- **AI Integration**: Install Claude Code/Cursor skill to let AI control the TUI via CLI

<img width="1728" height="1004" alt="Screenshot of oit tui" src="https://github.com/user-attachments/assets/05ee14bc-e22b-4203-a840-6e97fac71e70" />

## Installation

### From GitHub Releases (Recommended)

Download the latest release for macOS:

```bash
# Download and install the latest release
curl -L https://github.com/aha-app/overitall/releases/latest/download/oit-macos-arm64.tar.gz | tar xz
sudo mv oit /usr/local/bin/
```

For auto-updates to work, install to a user-writable location:

```bash
curl -L https://github.com/aha-app/overitall/releases/latest/download/oit-macos-arm64.tar.gz | tar xz
mv oit ~/.local/bin/
```

Or download manually from the [releases page](https://github.com/aha-app/overitall/releases).

### From Source

```bash
git clone https://github.com/aha-app/overitall.git
cd overitall
cargo build --release
```

The binary will be at `target/release/oit`. You can copy it to your PATH:

```bash
cp target/release/oit /usr/local/bin/
```

## Quick Start

1. Create a [Procfile](https://devcenter.heroku.com/articles/procfile) in your project directory:

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

# Use a different Procfile for initialization
oit --init -f Procfile.dev
```

3. Run `oit`:

```bash
oit
```

To start only specific processes:

```bash
oit web worker     # Start only web and worker processes
oit web            # Start only the web process
```

Or specify a custom config file or Procfile:

```bash
oit --config path/to/config.toml
oit -c path/to/config.toml

# Use a different Procfile (overrides config file setting)
oit -f Procfile.dev
oit --file Procfile.other
```

## Usage

### Keyboard Shortcuts

#### Navigation
- `↑` / `↓` - Select previous/next log line
- `Shift+↑` / `Shift+↓` - Extend selection (multi-select mode)
- `Enter` - Expand selected line (show full content in overlay)
- `Ctrl+B` / `Ctrl+F` - Page up/down (Vim-style)
- `PageUp` / `PageDown` - Page up/down
- `Home` / `End` - Jump to top/bottom
- `?` - Show help overlay
- `q` - Quit application

#### Modes
- `:` - Enter command mode
- `/` - Enter search mode
- `Esc` - Exit current mode, close overlays, or jump to latest logs
- `w` - Cycle display mode: compact → full → wrap
- `t` - Cycle timestamp display: seconds → milliseconds → off
- `p` - Cycle process panel: normal → summary → minimal


#### Batch Navigation
- `[` / `]` - Previous/next batch
- `+` / `-` - Increase/decrease batch window by 100ms

#### Clipboard & Batch Operations
- `c` - Copy selected line(s) to clipboard (with timestamp and process)
- `Shift+C` - Copy entire batch to clipboard (all lines in batch)
- `x` - Contextual copy (same process within ±1s of selected line)
- `b` - Focus on batch containing the selected line
- `Esc` - Clear multi-select (when in multi-select mode)

#### Mouse
- Click on a process in the sidebar to select it
- Scroll wheel to navigate logs
- **Tip**: Hold `Shift` while selecting text to use your terminal's native text selection (bypasses the TUI's mouse capture)

### Commands

All commands are entered by pressing `:` followed by the command.

#### Process Management

- `:s <name>` - Start a process
- `:r <name>` - Restart a process (or all processes if no name given)
- `:k <name>` - Kill (stop) a process
- `:q` / `:quit` / `:exit` - Quit the application

Example:
```
:r worker    # Restart the worker process
:r           # Restart all processes
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

#### Process Visibility

Hide or show logs from specific processes temporarily. This is useful when you want to focus on certain processes without permanently filtering their logs.

- `:hide <name>` - Hide logs from a specific process
- `:show <name>` - Show logs from a specific process
- `:hide all` - Hide all process logs
- `:show all` - Show all process logs
- `:only <name>` - Show only one process, hide all others

Examples:
```
:hide worker        # Hide logs from the worker process
:show worker        # Show logs from the worker process again
:hide all           # Hide all process logs
:show all           # Show all process logs
:only web           # Show only web logs, hide all others
```

When a process is hidden, it will be marked as `[Hidden]` in the process list, and its logs will not appear in the log viewer. Hidden processes are saved to the configuration file and persist across restarts.

#### Batch Navigation

Log lines that arrive within a short time window are grouped into "batches". This helps you see related log output together.

- `:nb` - Next batch
- `:pb` - Previous batch
- `:sb` - Toggle batch view mode (show only current batch)
- `:bw` - Show current batch window
- `:bw <milliseconds>` - Set batch window (default: 100ms)
- `:bw fast` / `:bw medium` / `:bw slow` - Presets: 100ms / 1000ms / 5000ms

You can also use `[` and `]` keys for quick batch navigation, or `+` and `-` to adjust the batch window.

The batch window determines how close in time log lines must be to be grouped together. Adjust it based on your application's logging patterns (e.g., `:bw 1000` for 1 second window).

#### Time Navigation

- `:goto HH:MM` or `:goto HH:MM:SS` - Jump to absolute time
- `:goto -5m` - Jump back 5 minutes from current selection
- `:goto +30s` - Jump forward 30 seconds from current selection
- `:g` - Short form of `:goto`

Relative time supports `s` (seconds), `m` (minutes), and `h` (hours). Navigation is relative to the currently selected line, or the last log if tailing.

#### Search

- `/` - Start search (filters logs as you type)
- `Enter` - In search mode: enter selection mode (selects the last match)
- `↑` / `↓` - Navigate between matches in selection mode
- `Enter` - In expanded view: show context around the selected log
- `Esc` - Step back through modes (selection → typing → exit)

The search filters logs in real-time as you type. Press Enter to enter selection mode where you can navigate through the filtered results with arrow keys. Press Esc to step back: from selection mode back to typing, or from typing mode to exit search completely.

#### Display

- `:color` - Toggle process coloring on/off (persists to config)

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

# Hidden processes (automatically saved when you hide/show processes)
hidden_processes = ["worker"]
```

### Configuration Options

- `procfile` - Path to your Procfile (required)
- `processes.<name>.log_file` - Path to the log file for a specific process (optional)
- `processes.<name>.status` - Custom status configuration (see below)
- `log_files` - Array of standalone log files to tail (see below)
- `filters.include` - Array of regex patterns to include
- `filters.exclude` - Array of regex patterns to exclude
- `hidden_processes` - Array of process names to hide from log viewer (automatically saved)
- `ignored_processes` - Array of process names to skip entirely (not started at all)
- `max_log_buffer_mb` - Maximum memory for log buffer in megabytes (default: 50)
- `batch_window_ms` - Batch grouping window in milliseconds (default: 100)
- `context_copy_seconds` - Time window for X (contextual copy) in seconds (default: 1.0)
- `disable_auto_update` - Set to `true` to disable auto-update checks (default: false)
- `compact_mode` - Set to `false` to show full log lines by default (default: true)
- `process_coloring` - Colorize process names in the log view (default: true)

### Standalone Log Files

You can tail log files that aren't associated with any process in your Procfile. This is useful for:
- Framework logs (Rails development.log, Sidekiq logs)
- System logs
- Any log file you want to view alongside your process output

```toml
# Tail standalone log files (not associated with any process)
[[log_files]]
name = "rails"
path = "log/development.log"

[[log_files]]
name = "sidekiq"
path = "log/sidekiq.log"
```

Standalone log files appear in the process list with a `[LOG]` indicator instead of a process status. You can hide/show them using the same commands as processes (`:hide rails`, `:show rails`).

Note: You cannot start, stop, or restart standalone log files - these commands are only for processes.

### Custom Process Status Labels

You can configure custom status labels that change based on log patterns. This is useful for showing meaningful status like "Starting", "Ready", "Migrating" instead of just "Running".

```toml
[processes.web]
log_file = "log/web.log"

[processes.web.status]
default = "Starting"

[[processes.web.status.transitions]]
pattern = "Listening on"
label = "Ready"
color = "green"

[[processes.web.status.transitions]]
pattern = "database migration"
label = "Migrating"
color = "yellow"
```

When the process starts, it shows the default status ("Starting"). When a log line matches a transition pattern, the status updates to the corresponding label with the specified color.

Available colors: `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`

The status resets to the default when the process is restarted.

### Process Coloring

Each process/log file name is shown in a distinct color in the log view, making it easier to visually distinguish logs from different sources. This is enabled by default.

```toml
# Disable colored process names (enabled by default)
process_coloring = false

# Optional: override specific process colors
[colors]
web = "green"
worker = "yellow"
rails = "cyan"
```

Available colors: `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`, plus light variants: `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`

You can also toggle coloring at runtime with the `:color` command, which persists the setting to your config file.

### Auto-Update

Overitall automatically checks for updates on every startup. When a new version is available, it will:

1. Download the update from GitHub releases
2. Replace the current binary
3. Restart with the new version

To skip the update check, use the `--no-update` flag:

```bash
oit --no-update
```

Or disable auto-update permanently in your config file:

```toml
disable_auto_update = true
```

### AI Integration (Claude Code / Cursor)

Install the AI skill to teach Claude Code and Cursor how to control the running TUI via CLI commands:

```bash
oit skill install
```

The skill is installed to `.claude/skills/oit/` (or `.cursor/skills/oit/`) and is automatically added to `.git/info/exclude` to prevent it from being committed to your repository.

Once installed, AI assistants can control the running TUI with commands like:

- `oit summary` - Get comprehensive status (processes, recent logs, errors)
- `oit errors --limit 10` - Get recent error logs
- `oit restart worker` - Restart a process
- `oit search "pattern"` - Search logs
- `oit freeze on` - Pause the display

This enables AI pair-programming workflows where the AI can investigate logs, restart processes, and manage filters while you watch the TUI.

### Memory Management

By default, Overitall limits the log buffer to 50 MB. When this limit is reached, the oldest logs are automatically evicted (First-In-First-Out).

Configure the buffer size in your `.overitall.toml`:

```toml
max_log_buffer_mb = 100  # Allow up to 100 MB of logs
```

The status bar shows current buffer usage and warns when eviction occurs. This prevents memory issues with long-running processes and high-volume logs.

### Status Bar Indicators

The status bar at the bottom of the screen shows:
- **Buffer usage**: Current memory usage and percentage
- **Line count**: Total number of log lines in buffer
- **Batch count**: Number of detected batches (or current batch info in batch view)
- **Mode indicator**: Shows the current viewing mode:
  - `[TAIL]` (green) - Following new logs in real-time
  - `[SCROLL]` (yellow) - Viewing history (scrolled up from bottom)
  - `[BATCH]` (blue) - Viewing a specific batch
  - `[TRACE]` (cyan) - Viewing a captured trace

When recording a manual trace, the status bar also shows a red `● REC` indicator with elapsed time.

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

See [ARCHITECTURE.md](ARCHITECTURE.md) for code structure and how to add features.

## Project Status

Overitall is under active development. Current features:

- Easy initialization with `--init` flag (automatically generates config from Procfile)
- Process management (start/stop/restart)
- Custom process status labels (show "Starting", "Ready", etc. based on log patterns)
- Standalone log files (tail log files not associated with any process)
- Log file tailing and interleaved viewing
- Filtering (include/exclude patterns)
- Process visibility toggle (hide/show logs from specific processes)
- Search with highlighting
- Batch detection and navigation
- Line selection and expanded view (view full content of long log lines)
- Multi-select mode (select multiple lines with Shift+arrow keys)
- Clipboard operations (copy lines and batches to system clipboard)
- Batch focus from selected line
- Dynamic batch window configuration (adjust batch grouping on-the-fly)
- Trace detection and filtering (find correlation IDs like UUIDs)
- Manual trace capture (record logs during a time window with `s` key)
- Display modes: compact (collapse `[key:value]` metadata), full, and wrap
- Configurable timestamp display (seconds, milliseconds, or hidden)
- Persistent configuration
- Help system
- Auto-update on startup (via gh CLI)
- AI integration via Claude Code/Cursor skill installation (`oit skill install`)

#### Trace Detection

Find correlation IDs (UUIDs, trace IDs, etc.) that appear multiple times in your logs:

- `:traces` - Scan logs for correlation IDs and show selection overlay
- `Enter` - Select a trace to filter logs to only that trace
- `[` / `]` - Expand trace view backward/forward in time (5 seconds per press)
- `Esc` - Exit trace view and return to normal log display

Traces are detected as tokens that:
- Look like correlation IDs (UUIDs, long numbers, long hex strings, etc.)
- Appear 3 or more times in the logs
- Don't span the entire log buffer (those are likely config values)

This is useful for debugging request flows through multi-service architectures.

#### Manual Trace Capture

Capture logs during a specific time window without needing correlation IDs:

- `s` - Start recording (status bar shows "● REC" with elapsed time)
- `s` - Press again to stop recording and enter trace view with captured logs
- `Esc` - Cancel recording (if pressed while recording) or exit trace view (after capture)
- `[` / `]` - Expand time window backward/forward (same as auto-detected traces)

This is useful when you want to isolate logs for a specific action (like clicking a button or running a command) without needing correlation IDs in your logs.

## Author

Created by [Jeremy Wells](https://github.com/jemmyw)

