use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Normal,
    Debug,
}

pub enum OutputEvent<'a> {
    StartupSummary {
        total_slices: usize,
        starting_labels: &'a [String],
    },
    SliceStarting {
        label: &'a str,
    },
    SliceFinishedNormally {
        label: &'a str,
        terminal_message: &'a str,
    },
    SliceRunning {
        label: &'a str,
        thread_id: &'a str,
        worktree_path: &'a Path,
    },
    SliceStillRunning {
        label: &'a str,
        elapsed_seconds: u64,
    },
    SliceNeedsAttention {
        label: &'a str,
        thread_id: Option<&'a str>,
        worktree_path: &'a Path,
        reason: &'a str,
    },
    NewSliceDiscovered {
        label: &'a str,
    },
    Warning {
        message: &'a str,
    },
    FatalStartupFailure {
        message: &'a str,
    },
}

pub trait OutputSink {
    fn emit(&mut self, event: OutputEvent<'_>);
}

pub struct TerminalOutput {
    printed_header: bool,
    mode: OutputMode,
}

impl TerminalOutput {
    pub fn new(mode: OutputMode) -> Self {
        Self {
            printed_header: false,
            mode,
        }
    }
}

impl OutputSink for TerminalOutput {
    fn emit(&mut self, event: OutputEvent<'_>) {
        let lines = render_event_lines(self.mode, self.printed_header, event);
        if let Some((stdout_lines, stderr_lines, printed_header)) = lines {
            self.printed_header = printed_header;
            for line in stdout_lines {
                println!("{line}");
            }
            for line in stderr_lines {
                eprintln!("{line}");
            }
        }
    }
}

fn render_event_lines(
    mode: OutputMode,
    printed_header: bool,
    event: OutputEvent<'_>,
) -> Option<(Vec<String>, Vec<String>, bool)> {
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut next_printed_header = printed_header;

    match event {
        OutputEvent::StartupSummary {
            total_slices,
            starting_labels,
        } => {
            if !printed_header {
                stdout_lines.push("Night Shift Coordinator v0.1.0".to_string());
                stdout_lines.push(String::new());
                next_printed_header = true;
            }
            stdout_lines.push(format!("Found {total_slices} slices in folder"));
            stdout_lines.push(format!(
                "Starting {} slices: {}",
                starting_labels.len(),
                starting_labels.join(", ")
            ));
            stdout_lines.push(String::new());
        }
        OutputEvent::SliceStarting { label } => {
            if mode == OutputMode::Debug {
                stdout_lines.push(format!("[{label}] starting"));
            }
        }
        OutputEvent::SliceFinishedNormally {
            label,
            terminal_message,
        } => {
            stdout_lines.push(format!("[{label}] finished normally"));
            if !terminal_message.trim().is_empty() {
                stdout_lines.push(format!("  final_message: {}", terminal_message.trim()));
            }
        }
        OutputEvent::SliceRunning {
            label,
            thread_id,
            worktree_path,
        } => {
            if mode == OutputMode::Debug {
                stdout_lines.push(format!("[{label}] running"));
                stdout_lines.push(format!("  thread_id: {thread_id}"));
                stdout_lines.push(format!("  worktree: {}", worktree_path.display()));
            }
        }
        OutputEvent::SliceStillRunning {
            label,
            elapsed_seconds,
        } => {
            if mode == OutputMode::Debug {
                stdout_lines.push(format!(
                    "[{label}] still running ({elapsed_seconds}s elapsed)"
                ));
            }
        }
        OutputEvent::SliceNeedsAttention {
            label,
            thread_id,
            worktree_path,
            reason,
        } => {
            stdout_lines.push(format!("[{label}] needs attention"));
            stdout_lines.push(format!("  reason: {}", reason.trim()));
            if let Some(thread_id) = thread_id.filter(|value| !value.is_empty()) {
                stdout_lines.push(format!("  thread_id: {thread_id}"));
            }
            stdout_lines.push(format!("  worktree: {}", worktree_path.display()));
        }
        OutputEvent::NewSliceDiscovered { label } => {
            stdout_lines.push(format!("Discovered new slice: {label}.md"));
        }
        OutputEvent::Warning { message } => {
            stdout_lines.push(format!("Warning: {message}"));
        }
        OutputEvent::FatalStartupFailure { message } => {
            stderr_lines.push(format!("Fatal startup failure: {message}"));
        }
    }

    if stdout_lines.is_empty() && stderr_lines.is_empty() {
        return None;
    }

    Some((stdout_lines, stderr_lines, next_printed_header))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{OutputEvent, OutputMode, render_event_lines};

    #[test]
    fn normal_mode_suppresses_debug_only_events() {
        let starting = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::SliceStarting { label: "01_slice" },
        );
        let running = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::SliceRunning {
                label: "01_slice",
                thread_id: "thread-123",
                worktree_path: Path::new("/tmp/worktree"),
            },
        );
        let heartbeat = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::SliceStillRunning {
                label: "01_slice",
                elapsed_seconds: 60,
            },
        );

        assert!(starting.is_none());
        assert!(running.is_none());
        assert!(heartbeat.is_none());
    }

    #[test]
    fn debug_mode_prints_debug_only_events() {
        let starting = render_event_lines(
            OutputMode::Debug,
            false,
            OutputEvent::SliceStarting { label: "01_slice" },
        )
        .unwrap();
        let running = render_event_lines(
            OutputMode::Debug,
            false,
            OutputEvent::SliceRunning {
                label: "01_slice",
                thread_id: "thread-123",
                worktree_path: Path::new("/tmp/worktree"),
            },
        )
        .unwrap();
        let heartbeat = render_event_lines(
            OutputMode::Debug,
            false,
            OutputEvent::SliceStillRunning {
                label: "01_slice",
                elapsed_seconds: 60,
            },
        )
        .unwrap();

        assert_eq!(starting.0, vec!["[01_slice] starting".to_string()]);
        assert_eq!(
            running.0,
            vec![
                "[01_slice] running".to_string(),
                "  thread_id: thread-123".to_string(),
                "  worktree: /tmp/worktree".to_string()
            ]
        );
        assert_eq!(
            heartbeat.0,
            vec!["[01_slice] still running (60s elapsed)".to_string()]
        );
    }
}
