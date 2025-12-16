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
Always run tests after making changes.

## Performance Changes
Any performance optimization MUST include before/after benchmarks. Create a simple benchmark that measures the specific operation being optimized, run it before and after the change, and document the results in the commit message and scratch.md. Without measured proof, we don't know if the "optimization" actually helped.

## Code Style
- Comment lightly, do not leave obvious code comments
- Don't put info about git status in .md files

## Reference
See plan.md for detailed feature specs and planned work.
