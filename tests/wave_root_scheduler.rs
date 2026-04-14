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
            OutputEvent::ProgramRootSummary { total_wave_roots } => {
                format!("program-waves:{total_wave_roots}")
            }
            OutputEvent::WaveRootStarting { index, total, name } => {
                format!("wave-root-start:{index}/{total}:{name}")
            }
            OutputEvent::WaveRootFinished {
                index,
                total,
                name,
                result,
            } => format!("wave-root-finished:{index}/{total}:{name}:{result}"),
            OutputEvent::WaveRootStopped {
                index,
                total,
                name,
                reason,
            } => format!("wave-root-stopped:{index}/{total}:{name}:{reason}"),
            OutputEvent::WaveDirRootSummary { total_wave_dirs } => {
                format!("wave-dirs:{total_wave_dirs}")
            }
            OutputEvent::WaveDirStarting { index, total, name } => {
                format!("wave-dir-start:{index}/{total}:{name}")
            }
            OutputEvent::WaveDirFinished {
                index,
                total,
                name,
                result,
            } => format!("wave-dir-finished:{index}/{total}:{name}:{result}"),
            OutputEvent::WaveDirStopped {
                index,
                total,
                name,
                reason,
            } => format!("wave-dir-stopped:{index}/{total}:{name}:{reason}"),
            OutputEvent::StartupSummary { total_slices, .. } => format!("startup:{total_slices}"),
            OutputEvent::SliceStarting { label } => format!("starting:{label}"),
            OutputEvent::SliceFinishedNormally { label, .. } => format!("finished:{label}"),
            OutputEvent::SliceRunning { label, .. } => format!("running:{label}"),
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
        let (_updates_tx, updates_rx) = mpsc::unbounded_channel::<RunUpdate>();
        let _ = tx.send(Ok(outcome));
        Ok(StartedRun {
            thread_id: format!("thread-{}", self.start_count.load(Ordering::SeqCst)),
            updates: updates_rx,
            completion: rx,
        })
    }
}

#[tokio::test]
async fn wave_roots_run_in_order_and_compose_wave_dirs() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("program");
    fs::create_dir_all(root.join("wave03_ruby").join("batch01_bundle_exec_rspec")).unwrap();
    fs::create_dir_all(root.join("wave03_ruby").join("batch02_bin_rspec")).unwrap();
    fs::create_dir_all(root.join("wave04_elixir").join("batch01_mix_test")).unwrap();
    fs::write(
        root.join("wave03_ruby")
            .join("batch01_bundle_exec_rspec")
            .join("01_bundle_exec_rspec.md"),
        "slice one",
    )
    .unwrap();
    fs::write(
        root.join("wave03_ruby")
            .join("batch02_bin_rspec")
            .join("01_bin_rspec.md"),
        "slice two",
    )
    .unwrap();
    fs::write(
        root.join("wave04_elixir")
            .join("batch01_mix_test")
            .join("01_mix_test.md"),
        "slice three",
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
            "add program",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

    let merged_sha = git_stdout(repo.path(), ["rev-parse", "origin/main"]);
    let client = FakeAppServerClient::new(vec![
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 501),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 502),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 503),
        },
    ]);
    let mut output = RecordingOutput::new();

    run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            target_root: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveRoots,
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
            "program/wave03_ruby/batch01_bundle_exec_rspec/01_bundle_exec_rspec.md".to_string(),
            "program/wave03_ruby/batch02_bin_rspec/01_bin_rspec.md".to_string(),
            "program/wave04_elixir/batch01_mix_test/01_mix_test.md".to_string(),
        ]
    );
    assert!(lines.iter().any(|line| line == "program-waves:2"));
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-root-start:1/2:wave03_ruby")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-root-finished:1/2:wave03_ruby:MERGED_SUCCESS")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "wave-root-start:2/2:wave04_elixir")
    );
}

#[tokio::test]
async fn wave_roots_stop_on_first_non_success_wave_root() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("program");
    fs::create_dir_all(root.join("wave03_ruby").join("batch01_first")).unwrap();
    fs::create_dir_all(root.join("wave04_elixir").join("batch01_second")).unwrap();
    fs::write(
        root.join("wave03_ruby")
            .join("batch01_first")
            .join("01_first.md"),
        "slice one",
    )
    .unwrap();
    fs::write(
        root.join("wave04_elixir")
            .join("batch01_second")
            .join("01_second.md"),
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
            "add program",
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
            target_root: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveRoots,
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
            .any(|line| line.starts_with("wave-root-stopped:1/2:wave03_ruby:"))
    );
}

#[tokio::test]
async fn wave_roots_allow_parallel_slices_inside_later_wave_dir() {
    let repo = init_git_repo_with_origin();
    let root = repo.path().join("program");
    fs::create_dir_all(root.join("wave03_ruby").join("batch01_first")).unwrap();
    fs::create_dir_all(root.join("wave04_elixir").join("batch01_parallel")).unwrap();
    fs::write(
        root.join("wave03_ruby")
            .join("batch01_first")
            .join("01_first.md"),
        "slice one",
    )
    .unwrap();
    fs::write(
        root.join("wave04_elixir")
            .join("batch01_parallel")
            .join("01_a.md"),
        "slice a",
    )
    .unwrap();
    fs::write(
        root.join("wave04_elixir")
            .join("batch01_parallel")
            .join("02_b.md"),
        "slice b",
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
            "add program",
        ],
    );
    git_ok(repo.path(), ["push", "origin", "main"]);

    let merged_sha = git_stdout(repo.path(), ["rev-parse", "origin/main"]);
    let client = FakeAppServerClient::new(vec![
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 601),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 602),
        },
        TerminalRunOutcome::FinishedPromptOwned {
            terminal_message: merged_success_message(&merged_sha, 603),
        },
    ]);
    let mut output = RecordingOutput::new();

    run(
        SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            target_root: root,
            prompt_template: "prompt".to_string(),
            schedule: ScheduleMode::WaveRoots,
        },
        &client,
        &mut output,
    )
    .await
    .unwrap();

    let starts = client.started_paths.lock().unwrap().clone();
    assert_eq!(starts.len(), 3);
    assert_eq!(
        starts[0],
        "program/wave03_ruby/batch01_first/01_first.md".to_string()
    );
    assert!(starts[1].starts_with("program/wave04_elixir/batch01_parallel/"));
    assert!(starts[2].starts_with("program/wave04_elixir/batch01_parallel/"));
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
