//! Claude Code / Cursor skill installation for AI integration.
//!
//! This module provides functionality to install a skill that teaches
//! AI assistants (Claude Code, Cursor) how to use oit's IPC commands.

use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// SKILL.md content - main skill definition
pub const SKILL_MD: &str = r#"---
name: oit
description: Control the OIT TUI process manager via IPC commands. Use when you need to: check process status, view/search logs, restart/kill/start processes, manage log filters, freeze display, get error summaries, or navigate log batches. The oit TUI must be running for commands to work.
allowed-tools: Bash(oit:*)
---

# OIT Process Manager Skill

OIT (Overitall) is a TUI combining process management (like overmind) with log viewing (like lnav). When the TUI is running, you can control it via CLI commands.

## Prerequisites

The oit TUI must be running in another terminal: `oit` or `oit -c overitall.toml`

## Quick Start

Check if TUI is running:
```bash
oit ping
```

Get comprehensive status (best first command):
```bash
oit summary
```

Get recent errors/warnings:
```bash
oit errors --limit 10
```

Search logs for specific text:
```bash
oit search "connection refused"
```

Get context around a log line (use ID from search/errors output):
```bash
oit context 12345 --before 10 --after 10
```

Restart a process:
```bash
oit restart worker
```

**Note:** `logs`, `search`, and `errors` are separate commands. Use `search` to find text, `errors` to find error-level logs, and `logs` to get recent lines without filtering.

For full command reference, see [COMMANDS.md](COMMANDS.md).
"#;

/// COMMANDS.md content - full command reference
pub const COMMANDS_MD: &str = r#"# OIT IPC Commands Reference

All commands require the oit TUI to be running in another terminal.

## Status & Info

### `oit ping`
Check if TUI is running. Returns "pong" if connected.

### `oit status`
Get TUI status including frozen state, process count, log count.

### `oit processes`
List all processes with their status (running/stopped/failed).

### `oit commands`
List all available IPC commands.

## Logs

**Important:** `logs`, `search`, and `errors` are separate commands. Do not combine their options.

### `oit logs [--limit N] [--offset N]`
Get recent log lines (no filtering). Each line includes an ID for reference.
- `--limit N` - Number of lines (default 100)
- `--offset N` - Skip first N lines

Example: `oit logs --limit 50`

### `oit search <pattern> [--limit N] [--case-sensitive]`
Search logs for a text pattern. Also highlights matches in TUI.
- `<pattern>` - Required text to search for
- `--limit N` - Max results
- `--case-sensitive` - Case sensitive matching

Example: `oit search "error connecting"` or `oit search "timeout" --limit 20`

### `oit errors [--limit N] [--level L] [--process P]`
Get error and warning logs (searches for error patterns automatically).
- `--limit N` - Max results
- `--level L` - Filter by level (error/warn/error_or_warning)
- `--process P` - Filter by process name

Example: `oit errors --limit 10` or `oit errors --process web`

## Navigation

### `oit select <id>`
Select and expand a specific log line by ID.

### `oit context <id> [--before N] [--after N]`
Get log lines surrounding a specific line by ID.
- `<id>` - Required log line ID (from logs/search/errors output)
- `--before N` - Lines before (default 5)
- `--after N` - Lines after (default 5)

Example: `oit context 12345 --before 10 --after 10`

### `oit goto <id>`
Scroll TUI view to a specific log line.

### `oit scroll <direction> [--lines N]`
Scroll the TUI view.
- `direction` - up/down/top/bottom
- `--lines N` - Number of lines for up/down

### `oit freeze [mode]`
Pause/resume log updates in TUI.
- `mode` - on/off/toggle (default: toggle)

### `oit batch <id> [--scroll]`
Get all log lines from the same batch as the given line.
- `--scroll` - Also scroll TUI to the batch

## Filters

### `oit filters`
List current log filters.

### `oit filter-add <pattern> [--exclude]`
Add a filter. Persists to config.
- `--exclude` - Filter OUT matching lines (default: filter IN)

### `oit filter-remove <pattern>`
Remove a filter by pattern. Persists to config.

### `oit filter-clear`
Remove all filters. Persists to config.

## Process Visibility

### `oit visibility`
List which processes are visible/hidden.

### `oit hide <process>`
Hide logs from a process (runtime only, not persisted).

### `oit show <process>`
Show logs from a hidden process (runtime only).

## Process Control

### `oit restart [name]`
Restart a process, or all processes if no name given.

### `oit kill <name>`
Kill (stop) a specific process.

### `oit start <name>`
Start a stopped process.

## AI-Optimized

### `oit summary`
Get comprehensive state summary including:
- All process statuses
- Recent log lines
- Recent errors/warnings
- Active filters
- View state

Best command to use first when investigating issues.
"#;

/// Check if a directory looks like it might benefit from skill installation
pub fn detect_ai_tool_directory() -> Option<&'static str> {
    if Path::new(".claude").is_dir() {
        Some(".claude")
    } else if Path::new(".cursor").is_dir() {
        Some(".cursor")
    } else {
        None
    }
}

/// Install the oit skill to .claude/skills/oit/
pub fn install_skill(base_dir: &str) -> Result<()> {
    let skill_dir = Path::new(base_dir).join("skills").join("oit");

    // Create the skill directory
    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create directory {:?}", skill_dir))?;

    // Write SKILL.md
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_MD)
        .with_context(|| format!("Failed to write {:?}", skill_path))?;

    // Write COMMANDS.md
    let commands_path = skill_dir.join("COMMANDS.md");
    fs::write(&commands_path, COMMANDS_MD)
        .with_context(|| format!("Failed to write {:?}", commands_path))?;

    Ok(())
}

/// Prompt user for skill installation (only works in TTY mode)
pub fn prompt_skill_install(ai_dir: &str) -> Result<bool> {
    // Skip prompting in tests (tests set this env var to avoid TTY issues)
    if std::env::var("OIT_TEST_NO_TTY").is_ok() {
        return Ok(false);
    }

    // Check if we're running in a TTY
    if !atty::is(atty::Stream::Stdin) {
        return Ok(false);
    }

    print!(
        "{} detected. Install oit skill for AI integration? [y/N] ",
        match ai_dir {
            ".claude" => "Claude Code",
            ".cursor" => "Cursor",
            _ => "AI tool",
        }
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_skill_md_content_has_required_fields() {
        assert!(SKILL_MD.contains("name: oit"));
        assert!(SKILL_MD.contains("description:"));
        assert!(SKILL_MD.contains("allowed-tools:"));
        assert!(SKILL_MD.contains("Bash(oit:*)"));
    }

    #[test]
    fn test_skill_md_has_quick_start() {
        assert!(SKILL_MD.contains("Quick Start"));
        assert!(SKILL_MD.contains("oit ping"));
        assert!(SKILL_MD.contains("oit summary"));
    }

    #[test]
    fn test_commands_md_has_all_command_categories() {
        assert!(COMMANDS_MD.contains("## Status & Info"));
        assert!(COMMANDS_MD.contains("## Logs"));
        assert!(COMMANDS_MD.contains("## Navigation"));
        assert!(COMMANDS_MD.contains("## Filters"));
        assert!(COMMANDS_MD.contains("## Process Visibility"));
        assert!(COMMANDS_MD.contains("## Process Control"));
        assert!(COMMANDS_MD.contains("## AI-Optimized"));
    }

    #[test]
    fn test_commands_md_has_key_commands() {
        assert!(COMMANDS_MD.contains("oit ping"));
        assert!(COMMANDS_MD.contains("oit status"));
        assert!(COMMANDS_MD.contains("oit processes"));
        assert!(COMMANDS_MD.contains("oit logs"));
        assert!(COMMANDS_MD.contains("oit search"));
        assert!(COMMANDS_MD.contains("oit errors"));
        assert!(COMMANDS_MD.contains("oit restart"));
        assert!(COMMANDS_MD.contains("oit kill"));
        assert!(COMMANDS_MD.contains("oit start"));
        assert!(COMMANDS_MD.contains("oit summary"));
    }

    #[test]
    fn test_install_skill_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join(".claude");
        fs::create_dir(&base_dir).unwrap();

        install_skill(base_dir.to_str().unwrap()).unwrap();

        let skill_path = base_dir.join("skills").join("oit").join("SKILL.md");
        let commands_path = base_dir.join("skills").join("oit").join("COMMANDS.md");

        assert!(skill_path.exists(), "SKILL.md should be created");
        assert!(commands_path.exists(), "COMMANDS.md should be created");

        let skill_content = fs::read_to_string(&skill_path).unwrap();
        assert!(skill_content.contains("name: oit"));

        let commands_content = fs::read_to_string(&commands_path).unwrap();
        assert!(commands_content.contains("OIT IPC Commands Reference"));
    }

    #[test]
    fn test_install_skill_creates_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join(".claude");
        // Don't create base_dir - install_skill should handle it
        fs::create_dir(&base_dir).unwrap();

        install_skill(base_dir.to_str().unwrap()).unwrap();

        let skill_dir = base_dir.join("skills").join("oit");
        assert!(skill_dir.is_dir(), "skills/oit directory should be created");
    }

    #[test]
    fn test_detect_ai_tool_directory_finds_claude() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::create_dir(".claude").unwrap();
        let result = detect_ai_tool_directory();
        assert_eq!(result, Some(".claude"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_detect_ai_tool_directory_finds_cursor() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::create_dir(".cursor").unwrap();
        let result = detect_ai_tool_directory();
        assert_eq!(result, Some(".cursor"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_detect_ai_tool_directory_prefers_claude() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::create_dir(".claude").unwrap();
        fs::create_dir(".cursor").unwrap();
        let result = detect_ai_tool_directory();
        assert_eq!(result, Some(".claude"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_detect_ai_tool_directory_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_ai_tool_directory();
        assert_eq!(result, None);

        std::env::set_current_dir(original_dir).unwrap();
    }
}
