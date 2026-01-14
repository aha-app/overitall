# Agent Guide

## Project
Rust TUI combining overmind (process management) + lnav (log viewing).

**Read [ARCHITECTURE.md](ARCHITECTURE.md)** for code structure and where to make changes.

## Testing
- Write tests for changes and new functionality
- Run `cargo test` after changes. Use `cargo insta review` to approve snapshot changes.

## Performance Changes
Any optimization MUST include before/after benchmarks. Document results in commit message and scratch.md.

## Code Style
- Comment lightly, no obvious code comments
- Don't put git status info in .md files

## Documentation
- Add new features in README.md
- Add commands and keys in the help system
- Update the man doc
- Document new architecture patterns very concisely in ARCHITECTURE.md, only write out how it works, do not add discussion on pros/cons

## Git
- Work on new features in feature branches
- Do not add Claude as a co-author or include any AI signature in commit messages.

## Reference
- [ARCHITECTURE.md](ARCHITECTURE.md) - code structure, how to add features
- [plan.md](plan.md) - feature specs
- [todo.md](todo.md) - priority list
