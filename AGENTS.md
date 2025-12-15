# Agent Guide

## Project
Rust TUI combining overmind (process management) + lnav (log viewing).

## Architecture
- Separation of concerns: log sources, parsing, storage, filtering, display are separate
- Trait-based extensibility for future log sources and parsers
- Dual timestamps: parsed (from log content) + arrival (when received)
- Circular buffer with VecDeque for memory bounds
- Batch grouping by arrival proximity

## Testing
Use ratatui's `TestBackend` for TUI testing (renders to in-memory buffer, works in CI).
Snapshot testing with `insta` crate (`cargo insta review` to approve changes).

## Code Style
- Comment lightly, do not leave obvious code comments
- Don't put info about git status in .md files

## Reference
See plan.md for detailed feature specs and planned work.
