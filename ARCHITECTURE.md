# Architecture

## Operations Pattern

Commands and events delegate to `src/operations/` modules instead of inline logic. This enables testability, extensibility (API/scripting), and consistency.

**Pattern:**
```rust
// operations/batch.rs
pub fn next_batch(app: &mut App, manager: &ProcessManager) -> bool { ... }

// event_handler.rs - just routes to operation
fn handle_next_batch(&mut self) {
    batch::next_batch(self.app, self.manager);
}
```

**Guidelines:**
1. New features: create operation first, wire to commands/events
2. Handlers should only call operations and set status messages
3. Operations return Result<T, String> for errors

## Module Structure

- `main.rs` - Entry point, event loop
- `command.rs` - Command parsing, CommandExecutor
- `event_handler.rs` - Keyboard routing
- `config.rs` / `procfile.rs` - Configuration
- `process.rs` - ProcessManager
- `ui.rs` - App state, TUI rendering
- `log/` - Log sources, parsing, buffering
- `operations/` - Business logic

## Testing

Use `TestBackend` for TUI tests, `insta` for snapshots.
