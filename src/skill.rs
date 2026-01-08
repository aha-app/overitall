//! Claude Code / Cursor skill installation for AI integration.
//!
//! This module provides functionality to install a skill that teaches
//! AI assistants (Claude Code, Cursor) how to use oit's IPC commands.

use anyhow::{Context, Result};
use std::fs;
use std::io::{self, BufRead, Write};
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

/// Add the skill directory to .git/info/exclude if in a git repo
fn add_to_git_exclude(skill_dir: &str) -> Result<()> {
    let git_dir = Path::new(".git");
    if !git_dir.is_dir() {
        return Ok(()); // Not a git repo, nothing to do
    }

    let info_dir = git_dir.join("info");
    let exclude_path = info_dir.join("exclude");

    // Entry to add (relative path from repo root)
    let entry = format!("{}/skills/oit/", skill_dir);

    // Check if entry already exists
    if exclude_path.exists() {
        let file = fs::File::open(&exclude_path)
            .with_context(|| format!("Failed to read {:?}", exclude_path))?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.trim() == entry {
                    return Ok(()); // Already excluded
                }
            }
        }
    }

    // Ensure .git/info directory exists
    fs::create_dir_all(&info_dir)
        .with_context(|| format!("Failed to create {:?}", info_dir))?;

    // Append the entry
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_path)
        .with_context(|| format!("Failed to open {:?} for writing", exclude_path))?;

    writeln!(file, "{}", entry)
        .with_context(|| format!("Failed to write to {:?}", exclude_path))?;

    Ok(())
}

/// Install skill via CLI command (with user feedback)
pub fn install_skill_command() -> Result<()> {
    let ai_dir = detect_ai_tool_directory();

    if ai_dir.is_none() {
        anyhow::bail!(
            "No .claude or .cursor directory found in current directory.\n\
             Create a .claude directory to use with Claude Code, or .cursor for Cursor."
        );
    }

    let ai_dir = ai_dir.unwrap();
    let skill_dir = Path::new(ai_dir).join("skills").join("oit");

    install_skill(ai_dir)?;

    // Add to git exclude (ignore errors - not critical)
    let _ = add_to_git_exclude(ai_dir);

    println!("Installed oit skill to {:?}", skill_dir);
    println!("\nFiles created:");
    println!("  - SKILL.md (main skill definition)");
    println!("  - COMMANDS.md (full command reference)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that change the current directory
    static CWD_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = CWD_MUTEX.lock().unwrap();
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
        let _guard = CWD_MUTEX.lock().unwrap();
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
        let _guard = CWD_MUTEX.lock().unwrap();
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
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_ai_tool_directory();
        assert_eq!(result, None);

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_add_to_git_exclude_creates_entry() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .git/info directory
        fs::create_dir_all(".git/info").unwrap();

        // Call add_to_git_exclude
        add_to_git_exclude(".claude").unwrap();

        // Check that the entry was added
        let exclude_content = fs::read_to_string(".git/info/exclude").unwrap();
        assert!(exclude_content.contains(".claude/skills/oit/"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_add_to_git_exclude_idempotent() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::create_dir_all(".git/info").unwrap();

        // Call twice
        add_to_git_exclude(".claude").unwrap();
        add_to_git_exclude(".claude").unwrap();

        // Should only have one entry
        let exclude_content = fs::read_to_string(".git/info/exclude").unwrap();
        let count = exclude_content.matches(".claude/skills/oit/").count();
        assert_eq!(count, 1, "Entry should only appear once");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_add_to_git_exclude_no_git_dir() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // No .git directory - should succeed silently
        let result = add_to_git_exclude(".claude");
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_install_skill_command_creates_files() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .claude directory
        fs::create_dir(".claude").unwrap();
        // Create .git directory for exclude test
        fs::create_dir_all(".git/info").unwrap();

        // Call install_skill_command
        install_skill_command().unwrap();

        // Check that skill files were created
        assert!(Path::new(".claude/skills/oit/SKILL.md").exists());
        assert!(Path::new(".claude/skills/oit/COMMANDS.md").exists());

        // Check that git exclude was updated
        let exclude_content = fs::read_to_string(".git/info/exclude").unwrap();
        assert!(exclude_content.contains(".claude/skills/oit/"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_install_skill_command_updates_existing_files() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create .claude directory with existing outdated skill
        fs::create_dir_all(".claude/skills/oit").unwrap();
        fs::write(".claude/skills/oit/SKILL.md", "old content").unwrap();
        fs::write(".claude/skills/oit/COMMANDS.md", "old content").unwrap();

        // Call install_skill_command
        install_skill_command().unwrap();

        // Check that files were updated with latest content
        let skill_content = fs::read_to_string(".claude/skills/oit/SKILL.md").unwrap();
        assert!(skill_content.contains("name: oit"), "SKILL.md should be updated with latest content");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_install_skill_command_no_ai_dir() {
        let _guard = CWD_MUTEX.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // No .claude or .cursor directory - should error
        let result = install_skill_command();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No .claude or .cursor directory"));

        std::env::set_current_dir(original_dir).unwrap();
    }
}
