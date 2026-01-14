# Architecture

## Overview

Overitall is a TUI with three main components:
- **App** (`ui/app.rs`) - UI state: selection, filters, overlays, view mode
- **ProcessManager** (`process.rs`) - spawns processes, collects logs into a shared buffer
- **Config** (`config.rs`) - persistent settings, loaded at startup, saved on changes

The main loop in `main.rs` uses event-driven refresh: it wakes immediately on terminal events or new logs, with frame-rate limiting (~60fps) to batch rapid updates.

## Data Flow

```
Terminal Events → EventHandler → Operations → App/ProcessManager/Config
                                     ↓
Log Files → ProcessManager → LogBuffer → App (for display)
                                     ↓
                              UI Draw ← App state
```

## Operations Pattern

All business logic lives in `operations/` modules. Event handlers and commands just route to operations.

**To add a new feature:**
1. Create an operation in `operations/` that modifies App/ProcessManager/Config
2. Wire it to a command in `command.rs` or key handler in `event_handler.rs`
3. Update help overlay if user-facing

**To add a new command:**
1. Add variant to `Command` enum in `command.rs`
2. Add parsing in `parse_command()`
3. Handle in `CommandExecutor::execute()` by calling an operation

**To add a new key binding:**
1. Add handler method in `event_handler.rs`
2. Call an operation from the handler

## Log System

- **LogBuffer** (`log/buffer.rs`) - circular buffer with memory limit, FIFO eviction
- **Dual timestamps** - each LogLine has parsed timestamp (from content) + arrival timestamp (when received)
- **Batch grouping** - lines arriving within `batch_window_ms` are grouped for navigation

## UI Layer

- **Overlays** (`ui/overlays/`) - modal views (help, expanded line, trace selection)
- **Widgets** (`ui/widgets/`) - stateless rendering (log viewer, process list, status bar)
- **App state** drives what's rendered; widgets read from App

## IPC System

External tools (AI agents, scripts) control oit via Unix socket IPC.

```
CLI (oit ping) → IpcClient → Unix Socket → IpcServer → IpcCommandHandler → Response
```

- **IpcServer** (`ipc/server.rs`) - Unix socket listener, non-blocking poll for commands
- **IpcClient** (`ipc/client.rs`) - connects to socket, sends requests, receives responses
- **IpcCommandHandler** (`ipc/handler.rs`) - processes requests, returns JSON responses
- **Protocol** (`ipc/protocol.rs`) - `IpcRequest` and `IpcResponse` types, newline-delimited JSON

Socket location: `.oit.sock` in the current working directory.

**To add a new IPC command:**
1. Add handler method in `IpcCommandHandler` (e.g., `handle_mycommand`)
2. Add match arm in `handle()` method
3. Add CLI subcommand in `cli.rs` if needed
4. Add tests

## Testing

Use `TestBackend` for TUI tests, `insta` for snapshots. Run `cargo test`, review with `cargo insta review`.

**Test helpers**: Some `ProcessManager` methods like `set_process_status_for_testing` and `reset_process_status` are marked `#[doc(hidden)] pub` because integration tests in `tests/` need access but `#[cfg(test)]` items aren't visible to them. A cleaner future approach would be a `test-helpers` feature flag with `#[cfg(any(test, feature = "test-helpers"))]`.
