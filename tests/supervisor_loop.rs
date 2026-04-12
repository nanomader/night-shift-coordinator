use std::collections::VecDeque;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use night_shift_coordinator::app_server::{
    AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome,
};
use night_shift_coordinator::error::Result;
use night_shift_coordinator::output::{OutputEvent, OutputSink};
use night_shift_coordinator::supervisor::{SupervisorOptions, run};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
            OutputEvent::SliceRunning {
                label, thread_id, ..
            } => format!("running:{label}:{thread_id}"),
            OutputEvent::SliceStillRunning {
                label,
                elapsed_seconds,
            } => format!("heartbeat:{label}:{elapsed_seconds}"),
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
    start_count: Arc<AtomicUsize>,
}

impl FakeAppServerClient {
    fn new(outcomes: Vec<TerminalRunOutcome>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into())),
            start_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl AppServerClient for FakeAppServerClient {
    async fn start_run(&self, _request: RunSliceRequest) -> Result<StartedRun> {
        self.start_count.fetch_add(1, Ordering::SeqCst);
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

#[cfg(unix)]
#[tokio::test]
async fn startup_unreadable_slice_fails_before_any_run_is_launched() {
    let repo = init_git_repo();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    let unreadable = slice_dir.join("01_unreadable.md");
    fs::write(&unreadable, "hidden").unwrap();
    let original_permissions = fs::metadata(&unreadable).unwrap().permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&unreadable, unreadable_permissions).unwrap();

    let client = FakeAppServerClient::new(vec![]);
    let start_count = Arc::clone(&client.start_count);
    let mut output = RecordingOutput::new();

    let result = run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: slice_dir,
            prompt_template: "prompt".to_string(),
        },
        &client,
        &mut output,
    )
    .await;

    fs::set_permissions(&unreadable, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    let lines = output.lines.lock().unwrap().clone();
    assert!(lines.iter().any(|line| line.starts_with("fatal:")));
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
