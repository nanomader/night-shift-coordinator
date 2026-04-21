use std::path::Path;

use crate::terminal_result::{DebugEvent, ProgressPhase, TerminalResultMarkers};

const PROGRAM_HEADER: &str = concat!("Night Shift Coordinator v", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Normal,
    Debug,
}

pub enum OutputEvent<'a> {
    ProgramRootSummary {
        total_wave_roots: usize,
    },
    WaveRootStarting {
        index: usize,
        total: usize,
        name: &'a str,
    },
    WaveRootFinished {
        index: usize,
        total: usize,
        name: &'a str,
        result: &'a str,
    },
    WaveRootStopped {
        index: usize,
        total: usize,
        name: &'a str,
        reason: &'a str,
    },
    WaveDirRootSummary {
        total_wave_dirs: usize,
    },
    WaveDirStarting {
        index: usize,
        total: usize,
        name: &'a str,
    },
    WaveDirFinished {
        index: usize,
        total: usize,
        name: &'a str,
        result: &'a str,
    },
    WaveDirStopped {
        index: usize,
        total: usize,
        name: &'a str,
        reason: &'a str,
    },
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
        terminal_markers: Option<&'a TerminalResultMarkers>,
    },
    SliceRunning {
        label: &'a str,
        thread_id: &'a str,
        worktree_path: &'a Path,
    },
    SliceProgress {
        label: &'a str,
        phase: ProgressPhase,
    },
    SliceWorkflowEvent {
        label: &'a str,
        event: DebugEvent,
    },
    SliceNeedsAttention {
        label: &'a str,
        thread_id: Option<&'a str>,
        worktree_path: &'a Path,
        reason: &'a str,
    },
    Warning {
        message: &'a str,
    },
    FatalStartupFailure {
        message: &'a str,
    },
    RunStopped {
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
        OutputEvent::ProgramRootSummary { total_wave_roots } => {
            if !printed_header {
                stdout_lines.push(PROGRAM_HEADER.to_string());
                stdout_lines.push(String::new());
                next_printed_header = true;
            }
            stdout_lines.push(format!(
                "Found {total_wave_roots} wave roots in program root"
            ));
            stdout_lines.push(String::new());
        }
        OutputEvent::WaveRootStarting { index, total, name } => {
            stdout_lines.push(format!("Starting wave root {index}/{total}: {name}"));
        }
        OutputEvent::WaveRootFinished {
            index,
            total,
            name,
            result,
        } => {
            stdout_lines.push(format!(
                "Wave root {index}/{total} finished: {name} ({result})"
            ));
        }
        OutputEvent::WaveRootStopped {
            index,
            total,
            name,
            reason,
        } => {
            stdout_lines.push(format!("Wave root {index}/{total} stopped: {name}"));
            stdout_lines.push(format!("  reason: {}", reason.trim()));
        }
        OutputEvent::WaveDirRootSummary { total_wave_dirs } => {
            if !printed_header {
                stdout_lines.push(PROGRAM_HEADER.to_string());
                stdout_lines.push(String::new());
                next_printed_header = true;
            }
            stdout_lines.push(format!("Found {total_wave_dirs} wave-dirs in wave root"));
            stdout_lines.push(String::new());
        }
        OutputEvent::WaveDirStarting { index, total, name } => {
            stdout_lines.push(format!("Starting wave-dir {index}/{total}: {name}"));
        }
        OutputEvent::WaveDirFinished {
            index,
            total,
            name,
            result,
        } => {
            stdout_lines.push(format!(
                "Wave-dir {index}/{total} finished: {name} ({result})"
            ));
        }
        OutputEvent::WaveDirStopped {
            index,
            total,
            name,
            reason,
        } => {
            stdout_lines.push(format!("Wave-dir {index}/{total} stopped: {name}"));
            stdout_lines.push(format!("  reason: {}", reason.trim()));
        }
        OutputEvent::StartupSummary {
            total_slices,
            starting_labels,
        } => {
            if !printed_header {
                stdout_lines.push(PROGRAM_HEADER.to_string());
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
            terminal_markers,
        } => {
            stdout_lines.push(format!("[{label}] finished normally"));
            if let Some(markers) = terminal_markers {
                let mut summary = format!("  result: {}", markers.result.as_display_str());
                if let Some(pr_number) = markers.pr_number {
                    summary.push_str(&format!(" pr={pr_number}"));
                }
                if let Some(merged_main_sha) = markers.merged_main_sha.as_deref() {
                    summary.push_str(&format!(" merged_main_sha={merged_main_sha}"));
                }
                stdout_lines.push(summary);
            }
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
        OutputEvent::SliceProgress { label, phase } => {
            if mode == OutputMode::Debug {
                stdout_lines.push(format!(
                    "[{label}] phase: {}",
                    phase.as_display_str().replace('_', " ")
                ));
            }
        }
        OutputEvent::SliceWorkflowEvent { label, event } => {
            if mode == OutputMode::Debug {
                stdout_lines.push(format!("[{label}] event: {}", event.as_display_str()));
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
        OutputEvent::Warning { message } => {
            stdout_lines.push(format!("Warning: {message}"));
        }
        OutputEvent::FatalStartupFailure { message } => {
            stderr_lines.push(format!("Fatal startup failure: {message}"));
        }
        OutputEvent::RunStopped { message } => {
            stderr_lines.push(format!("Run stopped: {message}"));
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

    use crate::terminal_result::{
        DebugEvent, ProgressPhase, TerminalResultKind, TerminalResultMarkers,
    };

    use super::{OutputEvent, OutputMode, PROGRAM_HEADER, render_event_lines};

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
        let phase = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::SliceProgress {
                label: "01_slice",
                phase: ProgressPhase::Planning,
            },
        );
        let event = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::SliceWorkflowEvent {
                label: "01_slice",
                event: DebugEvent::ReviewPollingStarted,
            },
        );

        assert!(starting.is_none());
        assert!(running.is_none());
        assert!(phase.is_none());
        assert!(event.is_none());
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
        let phase = render_event_lines(
            OutputMode::Debug,
            false,
            OutputEvent::SliceProgress {
                label: "01_slice",
                phase: ProgressPhase::WaitingForReview,
            },
        )
        .unwrap();
        let event = render_event_lines(
            OutputMode::Debug,
            false,
            OutputEvent::SliceWorkflowEvent {
                label: "01_slice",
                event: DebugEvent::ReviewRetryPosted,
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
            phase.0,
            vec!["[01_slice] phase: waiting for review".to_string()]
        );
        assert_eq!(
            event.0,
            vec!["[01_slice] event: review retry posted".to_string()]
        );
    }

    #[test]
    fn wave_dir_events_render_in_normal_mode() {
        let summary = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::WaveDirRootSummary { total_wave_dirs: 3 },
        )
        .unwrap();
        let starting = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::WaveDirStarting {
                index: 1,
                total: 3,
                name: "batch02_php_vendor_bin_phpunit",
            },
        )
        .unwrap();
        let finished = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::WaveDirFinished {
                index: 1,
                total: 3,
                name: "batch02_php_vendor_bin_phpunit",
                result: "MERGED_SUCCESS",
            },
        )
        .unwrap();

        assert_eq!(
            summary.0,
            vec![
                PROGRAM_HEADER.to_string(),
                String::new(),
                "Found 3 wave-dirs in wave root".to_string(),
                String::new()
            ]
        );
        assert_eq!(
            starting.0,
            vec!["Starting wave-dir 1/3: batch02_php_vendor_bin_phpunit".to_string()]
        );
        assert_eq!(
            finished.0,
            vec![
                "Wave-dir 1/3 finished: batch02_php_vendor_bin_phpunit (MERGED_SUCCESS)"
                    .to_string()
            ]
        );
    }

    #[test]
    fn wave_root_events_render_in_normal_mode() {
        let summary = render_event_lines(
            OutputMode::Normal,
            false,
            OutputEvent::ProgramRootSummary {
                total_wave_roots: 3,
            },
        )
        .unwrap();
        let starting = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::WaveRootStarting {
                index: 1,
                total: 3,
                name: "wave03_ruby",
            },
        )
        .unwrap();
        let finished = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::WaveRootFinished {
                index: 1,
                total: 3,
                name: "wave03_ruby",
                result: "MERGED_SUCCESS",
            },
        )
        .unwrap();

        assert_eq!(
            summary.0,
            vec![
                PROGRAM_HEADER.to_string(),
                String::new(),
                "Found 3 wave roots in program root".to_string(),
                String::new()
            ]
        );
        assert_eq!(
            starting.0,
            vec!["Starting wave root 1/3: wave03_ruby".to_string()]
        );
        assert_eq!(
            finished.0,
            vec!["Wave root 1/3 finished: wave03_ruby (MERGED_SUCCESS)".to_string()]
        );
    }

    #[test]
    fn finished_slice_renders_normalized_result_summary_when_markers_exist() {
        let rendered = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::SliceFinishedNormally {
                label: "01_slice",
                terminal_message: "done\nNSC_RESULT: MERGED_SUCCESS",
                terminal_markers: Some(&TerminalResultMarkers {
                    result: TerminalResultKind::MergedSuccess,
                    merged_main_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
                    pr_number: Some(42),
                }),
            },
        )
        .unwrap();

        assert_eq!(
            rendered.0,
            vec![
                "[01_slice] finished normally".to_string(),
                "  result: merged_success pr=42 merged_main_sha=0123456789abcdef0123456789abcdef01234567"
                    .to_string(),
                "  final_message: done\nNSC_RESULT: MERGED_SUCCESS".to_string(),
            ]
        );
    }

    #[test]
    fn run_stopped_renders_to_stderr() {
        let rendered = render_event_lines(
            OutputMode::Normal,
            true,
            OutputEvent::RunStopped {
                message: "worktree creation failed",
            },
        )
        .unwrap();

        assert!(rendered.0.is_empty());
        assert_eq!(
            rendered.1,
            vec!["Run stopped: worktree creation failed".to_string()]
        );
    }
}
