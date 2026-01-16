# Plan

## Build Commands
- `cargo build` - development build
- `cargo test` - run tests
- `cargo insta review` - review snapshot test changes

## Current Feature: Process Groups

Branch: `feature/process-groups`

Named groups of processes for batch operations. Config example:
```toml
[groups]
rails = ["puma", "workers"]
```

Commands work with group names: `:r rails`, `:k rails`, etc.
