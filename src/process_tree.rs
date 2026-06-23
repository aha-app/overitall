//! Process tree snapshot and rendering model.
//!
//! Collects the OS process table via the cross-platform `sysinfo` crate and
//! builds a tree of the descendants of each managed process root. The
//! collection layer is cached with a short TTL so the viewer can re-render
//! every frame without sampling the OS each time. The conversion and
//! tree-line generation are pure functions so they can be tested without
//! depending on live process state.

use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// A single row from the OS process table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcInfo {
    pub pid: i32,
    pub ppid: i32,
    pub command: String,
}

/// A managed process root to expand into a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedRoot {
    pub name: String,
    /// Human-readable status label (e.g. "running", "stopped").
    pub status: String,
    /// Root pid (the `sh -c` process group leader), if the process is running.
    pub pid: Option<i32>,
}

/// What a rendered tree line represents (drives styling in the widget).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeLineKind {
    /// Managed process header (name + status).
    Root,
    /// A process node in the descendant tree.
    Node,
    /// Informational note (not running, pid not found, etc.).
    Info,
}

/// A single rendered line of the process tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeLine {
    pub kind: TreeLineKind,
    pub text: String,
}

/// Convert a raw process listing into `ProcInfo` rows.
///
/// `raw` yields `(pid, parent_pid, command)` tuples using the platform's
/// native (unsigned) pid width. Pids that do not fit in `i32` are skipped,
/// and a missing or out-of-range parent is recorded as `0` (no parent).
fn to_proc_infos<I>(raw: I) -> Vec<ProcInfo>
where
    I: IntoIterator<Item = (u32, Option<u32>, String)>,
{
    raw.into_iter()
        .filter_map(|(pid, ppid, command)| {
            let pid = i32::try_from(pid).ok()?;
            let ppid = ppid.and_then(|p| i32::try_from(p).ok()).unwrap_or(0);
            Some(ProcInfo { pid, ppid, command })
        })
        .collect()
}

/// Build the renderable tree lines for the given roots and process table.
pub fn build_tree_lines(roots: &[ManagedRoot], procs: &[ProcInfo]) -> Vec<TreeLine> {
    if roots.is_empty() {
        return vec![TreeLine {
            kind: TreeLineKind::Info,
            text: "No managed processes".to_string(),
        }];
    }

    // Index children by parent pid, keeping each child list sorted by pid for
    // deterministic output.
    let mut children: std::collections::HashMap<i32, Vec<&ProcInfo>> =
        std::collections::HashMap::new();
    for proc in procs {
        children.entry(proc.ppid).or_default().push(proc);
    }
    for kids in children.values_mut() {
        kids.sort_by_key(|p| p.pid);
    }

    let by_pid: std::collections::HashMap<i32, &ProcInfo> =
        procs.iter().map(|p| (p.pid, p)).collect();

    let mut lines = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            lines.push(TreeLine {
                kind: TreeLineKind::Info,
                text: String::new(),
            });
        }
        lines.push(TreeLine {
            kind: TreeLineKind::Root,
            text: format!("{}  {}", root.name, root.status),
        });

        match root.pid {
            None => lines.push(TreeLine {
                kind: TreeLineKind::Info,
                text: "   (not running)".to_string(),
            }),
            Some(pid) => {
                let mut visited = std::collections::HashSet::new();
                push_node(&mut lines, &children, &by_pid, pid, "", true, &mut visited);
            }
        }
    }
    lines
}

/// Recursively render the subtree rooted at `pid`.
fn push_node(
    lines: &mut Vec<TreeLine>,
    children: &std::collections::HashMap<i32, Vec<&ProcInfo>>,
    by_pid: &std::collections::HashMap<i32, &ProcInfo>,
    pid: i32,
    prefix: &str,
    is_last: bool,
    visited: &mut std::collections::HashSet<i32>,
) {
    // Guard against pid reuse cycles.
    if !visited.insert(pid) {
        return;
    }

    let branch = if is_last { "└─ " } else { "├─ " };
    let command = by_pid
        .get(&pid)
        .map(|p| p.command.as_str())
        .unwrap_or("<unknown>");
    lines.push(TreeLine {
        kind: TreeLineKind::Node,
        text: format!("{}{}{}  (pid {})", prefix, branch, command, pid),
    });

    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
    if let Some(kids) = children.get(&pid) {
        let last_idx = kids.len().saturating_sub(1);
        for (i, kid) in kids.iter().enumerate() {
            push_node(
                lines,
                children,
                by_pid,
                kid.pid,
                &child_prefix,
                i == last_idx,
                visited,
            );
        }
    }
}

/// Display command for a process: prefer the full command line, falling back
/// to the process name when arguments are unavailable.
fn command_for(process: &sysinfo::Process) -> String {
    let cmd = process.cmd();
    if cmd.is_empty() {
        process.name().to_string_lossy().into_owned()
    } else {
        cmd.iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Snapshot the OS process table via `sysinfo`. The `System` is reused across
/// refreshes so repeated samples avoid reallocating the process map.
///
/// The default `refresh_processes` does not populate command lines, so we
/// request `cmd` explicitly. We also disable tasks so Linux threads do not
/// appear as child processes in the tree.
fn snapshot_processes(system: &mut System) -> Vec<ProcInfo> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cmd(UpdateKind::Always),
    );
    let raw = system.processes().values().map(|process| {
        (
            process.pid().as_u32(),
            process.parent().map(|ppid| ppid.as_u32()),
            command_for(process),
        )
    });
    to_proc_infos(raw)
}

/// Cached OS process table with a short refresh TTL.
///
/// The viewer calls `get()` on every frame; the underlying `sysinfo` sample
/// only runs when the cache is older than the TTL.
pub struct ProcessTreeCache {
    system: System,
    procs: Vec<ProcInfo>,
    last_refresh: Option<Instant>,
    ttl: Duration,
}

impl Default for ProcessTreeCache {
    fn default() -> Self {
        Self {
            system: System::new(),
            procs: Vec::new(),
            last_refresh: None,
            ttl: Duration::from_millis(750),
        }
    }
}

impl ProcessTreeCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached process table, refreshing it if the TTL has elapsed.
    pub fn get(&mut self) -> &[ProcInfo] {
        let stale = self
            .last_refresh
            .map(|t| t.elapsed() >= self.ttl)
            .unwrap_or(true);
        if stale {
            self.procs = snapshot_processes(&mut self.system);
            self.last_refresh = Some(Instant::now());
        }
        &self.procs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_raw_rows() {
        let raw = vec![
            (1u32, None, "/sbin/launchd".to_string()),
            (
                226u32,
                Some(1u32),
                "/usr/libexec/usermanagerd -t 15".to_string(),
            ),
        ];
        let procs = to_proc_infos(raw);
        assert_eq!(
            procs,
            vec![
                ProcInfo {
                    pid: 1,
                    // No parent reported -> recorded as 0.
                    ppid: 0,
                    command: "/sbin/launchd".to_string()
                },
                ProcInfo {
                    pid: 226,
                    ppid: 1,
                    command: "/usr/libexec/usermanagerd -t 15".to_string()
                },
            ]
        );
    }

    #[test]
    fn skips_pids_that_do_not_fit_i32() {
        // A pid beyond i32::MAX cannot be represented and is dropped.
        let raw = vec![
            (u32::MAX, Some(1u32), "huge".to_string()),
            (10u32, Some(5u32), "sh -c echo hi".to_string()),
        ];
        let procs = to_proc_infos(raw);
        assert_eq!(
            procs,
            vec![ProcInfo {
                pid: 10,
                ppid: 5,
                command: "sh -c echo hi".to_string()
            }]
        );
    }

    #[test]
    fn out_of_range_parent_recorded_as_zero() {
        let raw = vec![(10u32, Some(u32::MAX), "child".to_string())];
        let procs = to_proc_infos(raw);
        assert_eq!(procs[0].ppid, 0);
    }

    #[test]
    fn empty_roots_reports_no_processes() {
        let lines = build_tree_lines(&[], &[]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].kind, TreeLineKind::Info);
        assert_eq!(lines[0].text, "No managed processes");
    }

    #[test]
    fn stopped_root_renders_not_running() {
        let roots = vec![ManagedRoot {
            name: "web".to_string(),
            status: "stopped".to_string(),
            pid: None,
        }];
        let lines = build_tree_lines(&roots, &[]);
        assert_eq!(lines[0].kind, TreeLineKind::Root);
        assert_eq!(lines[0].text, "web  stopped");
        assert_eq!(lines[1].kind, TreeLineKind::Info);
        assert_eq!(lines[1].text, "   (not running)");
    }

    #[test]
    fn builds_descendant_tree() {
        let roots = vec![ManagedRoot {
            name: "web".to_string(),
            status: "running".to_string(),
            pid: Some(100),
        }];
        let procs = vec![
            ProcInfo {
                pid: 100,
                ppid: 1,
                command: "sh -c bin/rails server".to_string(),
            },
            ProcInfo {
                pid: 101,
                ppid: 100,
                command: "ruby bin/rails server".to_string(),
            },
            ProcInfo {
                pid: 103,
                ppid: 101,
                command: "puma worker 1".to_string(),
            },
            ProcInfo {
                pid: 102,
                ppid: 101,
                command: "puma worker 0".to_string(),
            },
        ];
        let lines = build_tree_lines(&roots, &procs);
        let text: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            text,
            vec![
                "web  running",
                "└─ sh -c bin/rails server  (pid 100)",
                "   └─ ruby bin/rails server  (pid 101)",
                // children sorted by pid: 102 before 103
                "      ├─ puma worker 0  (pid 102)",
                "      └─ puma worker 1  (pid 103)",
            ]
        );
    }

    #[test]
    fn root_pid_missing_from_table_shows_unknown() {
        let roots = vec![ManagedRoot {
            name: "web".to_string(),
            status: "running".to_string(),
            pid: Some(999),
        }];
        let lines = build_tree_lines(&roots, &[]);
        assert_eq!(lines[1].text, "└─ <unknown>  (pid 999)");
    }

    #[test]
    fn multiple_roots_separated_by_blank_line() {
        let roots = vec![
            ManagedRoot {
                name: "web".to_string(),
                status: "running".to_string(),
                pid: Some(100),
            },
            ManagedRoot {
                name: "worker".to_string(),
                status: "running".to_string(),
                pid: Some(200),
            },
        ];
        let procs = vec![
            ProcInfo {
                pid: 100,
                ppid: 1,
                command: "sh -c web".to_string(),
            },
            ProcInfo {
                pid: 200,
                ppid: 1,
                command: "sh -c worker".to_string(),
            },
        ];
        let lines = build_tree_lines(&roots, &procs);
        let blank = lines
            .iter()
            .filter(|l| l.kind == TreeLineKind::Info && l.text.is_empty())
            .count();
        assert_eq!(blank, 1, "expected one blank separator between two roots");
    }

    #[test]
    fn handles_pid_cycle_without_infinite_loop() {
        // Pathological: pid 100 and 101 are each other's parent.
        let roots = vec![ManagedRoot {
            name: "web".to_string(),
            status: "running".to_string(),
            pid: Some(100),
        }];
        let procs = vec![
            ProcInfo {
                pid: 100,
                ppid: 101,
                command: "a".to_string(),
            },
            ProcInfo {
                pid: 101,
                ppid: 100,
                command: "b".to_string(),
            },
        ];
        let lines = build_tree_lines(&roots, &procs);
        // Should terminate and render each node at most once.
        assert!(lines.len() <= 4);
    }
}
