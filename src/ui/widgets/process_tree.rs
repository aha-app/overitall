use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::process::{ProcessManager, ProcessStatus};
use crate::process_tree::{ManagedRoot, TreeLineKind, build_tree_lines};
use crate::ui::app::App;

/// Human-readable status label for a managed process.
fn status_label(status: &ProcessStatus) -> String {
    match status {
        ProcessStatus::Running => "running".to_string(),
        ProcessStatus::Stopped => "stopped".to_string(),
        ProcessStatus::Terminating => "terminating".to_string(),
        ProcessStatus::Restarting => "restarting".to_string(),
        ProcessStatus::Failed(msg) => format!("failed ({})", msg),
    }
}

/// Color used for a managed process status (matches the process list panel).
fn status_color(status: &ProcessStatus) -> Color {
    match status {
        ProcessStatus::Running => Color::Green,
        ProcessStatus::Stopped => Color::Red,
        ProcessStatus::Restarting => Color::Cyan,
        ProcessStatus::Terminating => Color::Magenta,
        ProcessStatus::Failed(_) => Color::Red,
    }
}

/// Collect managed roots sorted by name for deterministic display.
fn collect_roots(manager: &ProcessManager) -> Vec<(ManagedRoot, Color)> {
    let mut roots: Vec<(ManagedRoot, Color)> = manager
        .get_processes()
        .values()
        .map(|handle| {
            (
                ManagedRoot {
                    name: handle.name.clone(),
                    status: status_label(&handle.status),
                    pid: handle.root_pid(),
                },
                status_color(&handle.status),
            )
        })
        .collect();
    roots.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    roots
}

/// Draw the process tree viewer in the content area.
pub fn draw_process_tree(f: &mut Frame, area: Rect, manager: &ProcessManager, app: &mut App) {
    let roots_with_color = collect_roots(manager);

    // Map each root name to its status color so we can tint the header line.
    let root_colors: std::collections::HashMap<String, Color> = roots_with_color
        .iter()
        .map(|(root, color)| (root.name.clone(), *color))
        .collect();

    let roots: Vec<ManagedRoot> = roots_with_color.into_iter().map(|(root, _)| root).collect();

    let procs = app.process_tree_cache.get();
    let tree_lines = build_tree_lines(&roots, procs);

    let lines: Vec<Line> = tree_lines
        .iter()
        .map(|tl| match tl.kind {
            TreeLineKind::Root => {
                // Tint the header by the managed process status when known.
                let color = root_colors
                    .get(tl.text.split("  ").next().unwrap_or(""))
                    .copied()
                    .unwrap_or(Color::White);
                Line::styled(
                    tl.text.clone(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
            }
            TreeLineKind::Node => Line::styled(tl.text.clone(), Style::default()),
            TreeLineKind::Info => {
                Line::styled(tl.text.clone(), Style::default().fg(Color::DarkGray))
            }
        })
        .collect();

    // Compute the scrollable viewport. The block title consumes one line even
    // with Borders::NONE, matching the log viewer's accounting.
    let total_lines = lines.len() as u16;
    let viewport = area.height.saturating_sub(1);
    let max_scroll = total_lines.saturating_sub(viewport);

    // Clamp the stored offset against the current content and persist the
    // viewport so paging keys can use it on the next keypress.
    let scroll = app.display.process_tree_scroll.min(max_scroll);
    app.display.process_tree_scroll = scroll;
    app.display.process_tree_viewport = viewport;

    let paragraph = Paragraph::new(lines).scroll((scroll, 0)).block(
        Block::default()
            .borders(Borders::NONE)
            .title("Process Tree (P to return to logs)")
            .title_style(Style::default().add_modifier(Modifier::BOLD)),
    );

    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_lowercase_words() {
        assert_eq!(status_label(&ProcessStatus::Running), "running");
        assert_eq!(status_label(&ProcessStatus::Stopped), "stopped");
        assert_eq!(
            status_label(&ProcessStatus::Failed("Exit code: 1".to_string())),
            "failed (Exit code: 1)"
        );
    }

    #[test]
    fn collect_roots_sorted_by_name() {
        let mut manager = ProcessManager::new();
        manager.add_process("web".to_string(), "true".to_string(), None, None, None);
        manager.add_process("api".to_string(), "true".to_string(), None, None, None);
        manager.add_process("worker".to_string(), "true".to_string(), None, None, None);

        let roots = collect_roots(&manager);
        let names: Vec<&str> = roots.iter().map(|(r, _)| r.name.as_str()).collect();
        assert_eq!(names, vec!["api", "web", "worker"]);
        // Not started: no root pid.
        assert!(roots.iter().all(|(r, _)| r.pid.is_none()));
    }
}
