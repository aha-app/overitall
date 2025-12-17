# Agent Guide

## Project
Rust TUI combining overmind (process management) + lnav (log viewing).

**Read [ARCHITECTURE.md](ARCHITECTURE.md)** for code structure and where to make changes.

## Testing
Run `cargo test` after changes. Use `cargo insta review` to approve snapshot changes.

## Performance Changes
Any optimization MUST include before/after benchmarks. Document results in commit message and scratch.md.

## Code Style
- Comment lightly, no obvious code comments
- Don't put git status info in .md files

## Reference
- [ARCHITECTURE.md](ARCHITECTURE.md) - code structure, how to add features
- [plan.md](plan.md) - feature specs
- [todo.md](todo.md) - priority list
