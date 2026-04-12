use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use night_shift_coordinator::app_server::{
    AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome,
};
use night_shift_coordinator::cli::ScheduleMode;
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
            OutputEvent::SliceRunning { label, .. } => format!("running:{label}"),
            OutputEvent::SliceStillRunning { label, .. } => format!("heartbeat:{label}"),
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
    started_paths: Arc<Mutex<Vec<String>>>,
    start_count: Arc<AtomicUsize>,
}

impl FakeAppServerClient {
    fn new(outcomes: Vec<TerminalRunOutcome>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into())),
            started_paths: Arc::new(Mutex::new(Vec::new())),
            start_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl AppServerClient for FakeAppServerClient {
    async fn start_run(&self, request: RunSliceRequest) -> Result<StartedRun> {
        self.start_count.fetch_add(1, Ordering::SeqCst);
        self.started_paths
            .lock()
            .unwrap()
            .push(request.slice_repo_relative_path.display().to_string());
        let outcome = self.outcomes.lock().unwrap().pop_front().unwrap();
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(outcome));
        Ok(StartedRun {
            thread_id: format!("thread-{}", self.start_count.load(Ordering::SeqCst)),
            completion: rx,
        })
    }
}

#[tokio::test]
async fn wave_dirs_run_in_order_and_unlock_after_success() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("waves");
    fs::create_dir_all(root.join("batch02_php_vendor_bin_phpunit")).unwrap();
    fs::create_dir_all(root.join("batch03_composer_test_phpunit")).unwrap();
    fs::write(
        root.join("batch02_php_vendor_bin_phpunit")
            .join("01_vendor_bin_phpunit.md"),
        "slice one",
    )
    .unwrap();
    fs::write(
        root.join("batch03_composer_test_phpunit")
            .join("01_composer_test_phpunit.md"),
        "slice two",
    )
    .unwrap();
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
            "add waves",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

    let merged_sha = git_stdout(repo.path(), ["rev-parse", "origin/main"]);
    let client = FakeAppServerClient::new(vec![
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 354),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 355),
        },
    ]);
    let mut output = RecordingOutput::new();

    run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveDirs,
        },
        &client,
        &mut output,
    )
    .await
    .unwrap();

    let starts = client.started_paths.lock().unwrap().clone();
    let lines = output.lines.lock().unwrap().clone();

    assert_eq!(
        starts,
        vec![
            "waves/batch02_php_vendor_bin_phpunit/01_vendor_bin_phpunit.md".to_string(),
            "waves/batch03_composer_test_phpunit/01_composer_test_phpunit.md".to_string(),
        ]
    );
    assert!(lines.iter().any(|line| line == "waves:2"));
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-start:1/2:batch02_php_vendor_bin_phpunit")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-finished:1/2:batch02_php_vendor_bin_phpunit:MERGED_SUCCESS")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-start:2/2:batch03_composer_test_phpunit")
    );
}

#[tokio::test]
async fn wave_dirs_stop_on_manual_followup_without_starting_next_wave() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("waves");
    fs::create_dir_all(root.join("batch02_first")).unwrap();
    fs::create_dir_all(root.join("batch03_second")).unwrap();
    fs::write(root.join("batch02_first").join("01_first.md"), "slice one").unwrap();
    fs::write(
        root.join("batch03_second").join("01_second.md"),
        "slice two",
    )
    .unwrap();
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
            "add waves",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

    let client = FakeAppServerClient::new(vec![TerminalRunOutcome::FinishedPromptOwned {
        terminal_message: "not merged\nNSC_RESULT: MANUAL_FOLLOWUP\n".to_string(),
    }]);
    let mut output = RecordingOutput::new();

    let result = run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveDirs,
        },
        &client,
        &mut output,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(client.start_count.load(Ordering::SeqCst), 1);
    let lines = output.lines.lock().unwrap().clone();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("wave-stopped:1/2:batch02_first:"))
    );
}

#[tokio::test]
async fn wave_dirs_allow_parallel_slices_inside_one_wave_dir() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("waves");
    fs::create_dir_all(root.join("batch02_first")).unwrap();
    fs::create_dir_all(root.join("batch03_second")).unwrap();
    fs::write(root.join("batch02_first").join("01_a.md"), "slice a").unwrap();
    fs::write(root.join("batch02_first").join("02_b.md"), "slice b").unwrap();
    fs::write(root.join("batch03_second").join("01_c.md"), "slice c").unwrap();
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
            "add waves",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

    let merged_sha = git_stdout(repo.path(), ["rev-parse", "origin/main"]);
    let client = FakeAppServerClient::new(vec![
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 400),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 401),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 402),
        },
    ]);
    let mut output = RecordingOutput::new();

    run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveDirs,
        },
        &client,
        &mut output,
    )
    .await
    .unwrap();

    let starts = client.started_paths.lock().unwrap().clone();
    assert_eq!(starts.len(), 3);
    assert!(starts[0].starts_with("waves/batch02_first/"));
    assert!(starts[1].starts_with("waves/batch02_first/"));
    assert_eq!(starts[2], "waves/batch03_second/01_c.md");
}

fn merged_success_message(sha: &str, pr_number: u64) -> String {
    format!("done\nNSC_RESULT: MERGED_SUCCESS\nNSC_MERGED_MAIN_SHA: {sha}\nNSC_PR: {pr_number}\n")
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

fn git_stdout<const N: usize>(repo_root: &Path, args: [&str; N]) -> String {
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
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
