use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use night_shift_coordinator::app_server::{
    AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome,
};
use night_shift_coordinator::error::Result;
use night_shift_coordinator::output::{OutputEvent, OutputSink};
use night_shift_coordinator::supervisor::{SupervisorOptions, run};
use tempfile::TempDir;
use tokio::sync::oneshot;

struct RecordingOutput {
    lines: Arc<Mutex<Vec<String>>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl OutputSink for RecordingOutput {
    fn emit(&mut self, event: OutputEvent<'_>) {
        let line = match event {
            OutputEvent::StartupSummary { total_slices, .. } => format!("startup:{total_slices}"),
            OutputEvent::SliceStarting { label } => format!("starting:{label}"),
            OutputEvent::SliceFinishedNormally { label, .. } => format!("finished:{label}"),
            OutputEvent::SliceNeedsAttention { label, .. } => format!("attention:{label}"),
            OutputEvent::NewSliceDiscovered { label } => format!("new:{label}"),
            OutputEvent::Warning { message } => format!("warning:{message}"),
            OutputEvent::FatalStartupFailure { message } => format!("fatal:{message}"),
        };
        self.lines.lock().unwrap().push(line);
    }
}

struct FakeAppServerClient {
    outcomes: Arc<Mutex<VecDeque<TerminalRunOutcome>>>,
}

impl FakeAppServerClient {
    fn new(outcomes: Vec<TerminalRunOutcome>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into())),
        }
    }
}

#[async_trait]
impl AppServerClient for FakeAppServerClient {
    async fn start_run(&self, _request: RunSliceRequest) -> Result<StartedRun> {
        let outcome = self.outcomes.lock().unwrap().pop_front().unwrap();
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(outcome));
        Ok(StartedRun {
            thread_id: "thread-123".to_string(),
            completion: rx,
        })
    }
}

#[tokio::test]
async fn starts_next_slice_when_a_slot_frees_up() {
    let repo = init_git_repo();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    for idx in 1..=4 {
        fs::write(
            slice_dir.join(format!("{idx:02}_slice.md")),
            format!("slice {idx}"),
        )
        .unwrap();
    }

    let client = FakeAppServerClient::new(vec![
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: "done".to_string(),
        };
        4
    ]);
    let mut output = RecordingOutput::new();

    run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: slice_dir,
            prompt_template: "prompt".to_string(),
        },
        &client,
        &mut output,
    )
    .await
    .unwrap();

    let lines = output.lines.lock().unwrap().clone();
    assert!(lines.iter().any(|line| line == "starting:04_slice"));
}

fn init_git_repo() -> TempDir {
    let temp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    fs::write(temp.path().join("README.md"), "test").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    temp
}
