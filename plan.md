# Plan

## Build Commands
- `cargo build` - development build
- `cargo test` - run tests
- `cargo insta review` - review snapshot test changes

## Current Feature: Multi-Select Mode

Branch: `feature/multi-select`

Selection model uses anchor+end range approach:
- `selection_anchor`: ID of line where Shift+arrow started
- `selection_end`: Current cursor position (moves with each Shift+arrow)
- Selection includes all lines between anchor and end in display order

This is simpler than tracking individual IDs and matches how text editor selection typically works.
