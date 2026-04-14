use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use night_shift_coordinator::app_server::{
    AppServerClient, RunSliceRequest, RunUpdate, StartedRun, TerminalRunOutcome,
};
use night_shift_coordinator::cli::ScheduleMode;
use night_shift_coordinator::error::Result;
use night_shift_coordinator::output::{OutputEvent, OutputSink};
use night_shift_coordinator::supervisor::{SupervisorOptions, run};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;
use tokio::sync::{mpsc, oneshot};

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
            OutputEvent::WaveRootSummary { total_wave_dirs } => format!("waves:{total_wave_dirs}"),
            OutputEvent::WaveDirStarting { index, total, name } => {
                format!("wave-start:{index}/{total}:{name}")
            }
            OutputEvent::WaveDirFinished {
                index,
                total,
                name,
                result,
            } => format!("wave-finished:{index}/{total}:{name}:{result}"),
            OutputEvent::WaveDirStopped {
                index,
                total,
                name,
                reason,
            } => format!("wave-stopped:{index}/{total}:{name}:{reason}"),
            OutputEvent::StartupSummary { total_slices, .. } => format!("startup:{total_slices}"),
            OutputEvent::SliceStarting { label } => format!("starting:{label}"),
            OutputEvent::SliceFinishedNormally { label, .. } => format!("finished:{label}"),
            OutputEvent::SliceRunning {
                label, thread_id, ..
            } => format!("running:{label}:{thread_id}"),
            OutputEvent::SliceProgress { label, phase } => {
                format!("phase:{label}:{}", phase.as_display_str())
            }
            OutputEvent::SliceWorkflowEvent { label, event } => {
                format!("event:{label}:{}", event.as_display_str())
            }
            OutputEvent::SliceNeedsAttention { label, .. } => format!("attention:{label}"),
            OutputEvent::Warning { message } => format!("warning:{message}"),
            OutputEvent::FatalStartupFailure { message } => format!("fatal:{message}"),
            OutputEvent::RunStopped { message } => format!("run-stopped:{message}"),
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
        let (_updates_tx, updates_rx) = mpsc::unbounded_channel::<RunUpdate>();
        let _ = tx.send(Ok(outcome));
        Ok(StartedRun {
            thread_id: "thread-123".to_string(),
            updates: updates_rx,
            completion: rx,
        })
    }
}

#[tokio::test]
async fn starts_next_slice_when_a_slot_frees_up() {
    let repo = init_git_repo_with_origin();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    for idx in 1..=4 {
        fs::write(
            slice_dir.join(format!("{idx:02}_slice.md")),
            format!("slice {idx}"),
        )
        .unwrap();
    }
    git_ok(repo.path(), ["add", "."]);
    git_ok(
        repo.path(),
        [
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            "add slices",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

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
            schedule: ScheduleMode::Flat,
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
    let repo = init_git_repo_with_origin();
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
            schedule: ScheduleMode::Flat,
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

#[tokio::test]
async fn startup_fails_when_origin_main_is_unavailable() {
    let repo = TempDir::new().unwrap();
    git_ok(repo.path(), ["init", "--initial-branch=main"]);
    fs::write(repo.path().join("README.md"), "test").unwrap();
    git_ok(repo.path(), ["add", "."]);
    git_ok(
        repo.path(),
        [
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            "init",
        ],
    );

    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    fs::write(slice_dir.join("01_slice.md"), "slice").unwrap();

    let client = FakeAppServerClient::new(vec![]);
    let start_count = Arc::clone(&client.start_count);
    let mut output = RecordingOutput::new();

    let result = run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: slice_dir,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::Flat,
        },
        &client,
        &mut output,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    let lines = output.lines.lock().unwrap().clone();
    assert!(lines.iter().any(|line| line.starts_with("fatal:")));
}

fn init_git_repo_with_origin() -> TempDir {
    let temp = TempDir::new().unwrap();
    git_ok(temp.path(), ["init", "--initial-branch=main"]);
    fs::write(temp.path().join("README.md"), "test").unwrap();
    git_ok(temp.path(), ["add", "."]);
    git_ok(
        temp.path(),
        [
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            "init",
        ],
    );
    let origin_path = temp.path().join("_origin.git");
    fs::create_dir_all(&origin_path).unwrap();
    git_ok(
        &origin_path,
        ["init", "--bare", "--initial-branch=main", "."],
    );
    git_ok(
        temp.path(),
        ["remote", "add", "origin", origin_path.to_str().unwrap()],
    );
    git_ok(temp.path(), ["push", "-u", "origin", "main"]);
    temp
}

fn git_ok<const N: usize>(repo_root: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={}, stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
