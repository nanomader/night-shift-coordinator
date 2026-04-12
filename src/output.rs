use std::path::Path;

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

#[derive(Default)]
pub struct TerminalOutput {
    printed_header: bool,
}

impl OutputSink for TerminalOutput {
    fn emit(&mut self, event: OutputEvent<'_>) {
        match event {
            OutputEvent::StartupSummary {
                total_slices,
                starting_labels,
            } => {
                if !self.printed_header {
                    println!("Night Shift Coordinator v0.1.0");
                    println!();
                    self.printed_header = true;
                }
                println!("Found {total_slices} slices in folder");
                println!(
                    "Starting {} slices: {}",
                    starting_labels.len(),
                    starting_labels.join(", ")
                );
                println!();
            }
            OutputEvent::SliceStarting { label } => {
                println!("[{label}] starting");
            }
            OutputEvent::SliceFinishedNormally {
                label,
                terminal_message,
            } => {
                println!("[{label}] finished normally");
                if !terminal_message.trim().is_empty() {
                    println!("  final_message: {}", terminal_message.trim());
                }
            }
            OutputEvent::SliceNeedsAttention {
                label,
                thread_id,
                worktree_path,
                reason,
            } => {
                println!("[{label}] needs attention");
                println!("  reason: {}", reason.trim());
                if let Some(thread_id) = thread_id.filter(|value| !value.is_empty()) {
                    println!("  thread_id: {thread_id}");
                }
                println!("  worktree: {}", worktree_path.display());
            }
            OutputEvent::NewSliceDiscovered { label } => {
                println!("Discovered new slice: {label}.md");
            }
            OutputEvent::Warning { message } => {
                println!("Warning: {message}");
            }
            OutputEvent::FatalStartupFailure { message } => {
                eprintln!("Fatal startup failure: {message}");
            }
        }
    }
}
