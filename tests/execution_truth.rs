use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use night_shift_coordinator::app_server::{
    AppServerClient, RunSliceRequest, RunUpdate, StartedRun, TerminalRunOutcome,
};
use night_shift_coordinator::cli::ScheduleMode;
use night_shift_coordinator::error::Result;
use night_shift_coordinator::output::{OutputEvent, OutputSink};
use night_shift_coordinator::supervisor::{SupervisorOptions, run};
use night_shift_coordinator::terminal_result::{DebugEvent, ProgressPhase};
use tempfile::TempDir;
use tokio::sync::{mpsc, oneshot};
use tokio::task::LocalSet;

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

struct ImmediateClient {
    start_count: Arc<AtomicUsize>,
}

impl ImmediateClient {
    fn new() -> Self {
        Self {
            start_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl AppServerClient for ImmediateClient {
    async fn start_run(&self, _request: RunSliceRequest) -> Result<StartedRun> {
        self.start_count.fetch_add(1, Ordering::SeqCst);
        let (_tx, rx) = oneshot::channel();
        let (_updates_tx, updates_rx) = mpsc::unbounded_channel::<RunUpdate>();
        Ok(StartedRun {
            thread_id: "thread-unused".to_string(),
            updates: updates_rx,
            completion: rx,
        })
    }
}

struct ControlledClient {
    start_count: Arc<AtomicUsize>,
    completions: Arc<Mutex<Vec<oneshot::Sender<Result<TerminalRunOutcome>>>>>,
    update_senders: Arc<Mutex<Vec<mpsc::UnboundedSender<RunUpdate>>>>,
    worktree_paths: Arc<Mutex<Vec<std::path::PathBuf>>>,
}

impl ControlledClient {
    fn new() -> Self {
        Self {
            start_count: Arc::new(AtomicUsize::new(0)),
            completions: Arc::new(Mutex::new(Vec::new())),
            update_senders: Arc::new(Mutex::new(Vec::new())),
            worktree_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn complete_one(&self, outcome: TerminalRunOutcome) {
        let sender = self.completions.lock().unwrap().remove(0);
        let _ = sender.send(Ok(outcome));
    }

    fn send_progress(&self, phase: ProgressPhase) {
        let sender = self.update_senders.lock().unwrap()[0].clone();
        let _ = sender.send(RunUpdate::Progress(phase));
    }

    fn send_event(&self, event: DebugEvent) {
        let sender = self.update_senders.lock().unwrap()[0].clone();
        let _ = sender.send(RunUpdate::Event(event));
    }
}

#[async_trait]
impl AppServerClient for ControlledClient {
    async fn start_run(&self, request: RunSliceRequest) -> Result<StartedRun> {
        let run_id = self.start_count.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = oneshot::channel();
        let (updates_tx, updates_rx) = mpsc::unbounded_channel();
        self.completions.lock().unwrap().push(tx);
        self.update_senders.lock().unwrap().push(updates_tx);
        self.worktree_paths
            .lock()
            .unwrap()
            .push(request.worktree_path);
        Ok(StartedRun {
            thread_id: format!("thread-{run_id}"),
            updates: updates_rx,
            completion: rx,
        })
    }
}

#[tokio::test]
async fn flat_mode_rejects_local_only_slice_before_launch() {
    let repo = init_repo_with_origin();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    fs::write(slice_dir.join("01_local_only.md"), "slice").unwrap();

    let client = ImmediateClient::new();
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
    assert!(
        lines
            .iter()
            .any(|line| line.contains("slice file is missing from execution base"))
    );
}

#[tokio::test]
async fn flat_mode_does_not_admit_new_slice_files_after_startup() {
    let repo = init_repo_with_origin();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    fs::write(slice_dir.join("01_first.md"), "slice").unwrap();
    commit_and_push(repo.path(), ["slices/01_first.md"], "add first slice");

    let client = Arc::new(ControlledClient::new());
    let start_count = Arc::clone(&client.start_count);
    let output = RecordingOutput::new();
    let lines = Arc::clone(&output.lines);
    let repo_root = repo.path().to_path_buf();
    let run_slice_dir = slice_dir.clone();
    let run_client = Arc::clone(&client);
    let local = LocalSet::new();

    local
        .run_until(async move {
            let handle = tokio::task::spawn_local(async move {
                let mut output = RecordingOutput { lines };
                run(
                    SupervisorOptions {
                        repo_root,
                        slice_folder: run_slice_dir,
                        prompt_template: "prompt".to_string(),
                        schedule: ScheduleMode::Flat,
                    },
                    run_client.as_ref(),
                    &mut output,
                )
                .await
            });

            wait_for(|| start_count.load(Ordering::SeqCst) == 1).await;
            fs::write(slice_dir.join("02_late.md"), "late").unwrap();
            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });

            handle.await.unwrap().unwrap();
            assert_eq!(start_count.load(Ordering::SeqCst), 1);
        })
        .await;
}

#[tokio::test]
async fn wave_dir_membership_is_frozen_once_the_wave_starts() {
    let repo = init_repo_with_origin();
    let root = repo.path().join("waves");
    fs::create_dir_all(root.join("batch02_first")).unwrap();
    fs::write(root.join("README.md"), "wave root").unwrap();
    fs::write(root.join("batch02_first").join("01_first.md"), "slice").unwrap();
    commit_and_push(
        repo.path(),
        ["waves/README.md", "waves/batch02_first/01_first.md"],
        "add wave",
    );

    let merged_sha = git_stdout(repo.path(), ["rev-parse", "origin/main"]);
    let client = Arc::new(ControlledClient::new());
    let start_count = Arc::clone(&client.start_count);
    let output = RecordingOutput::new();
    let lines = Arc::clone(&output.lines);
    let repo_root = repo.path().to_path_buf();
    let run_root = root.clone();
    let run_client = Arc::clone(&client);
    let local = LocalSet::new();

    local
        .run_until(async move {
            let handle = tokio::task::spawn_local(async move {
                let mut output = RecordingOutput { lines };
                run(
                    SupervisorOptions {
                        repo_root,
                        slice_folder: run_root,
                        prompt_template: "prompt".to_string(),
                        schedule: ScheduleMode::WaveDirs,
                    },
                    run_client.as_ref(),
                    &mut output,
                )
                .await
            });

            wait_for(|| start_count.load(Ordering::SeqCst) == 1).await;
            fs::write(root.join("batch02_first").join("02_late.md"), "late").unwrap();
            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: format!(
                    "done\nNSC_RESULT: MERGED_SUCCESS\nNSC_MERGED_MAIN_SHA: {merged_sha}\nNSC_PR: 42\n"
                ),
            });

            handle.await.unwrap().unwrap();
            assert_eq!(start_count.load(Ordering::SeqCst), 1);
        })
        .await;
}

#[tokio::test]
async fn progress_markers_update_debug_output_and_recovery() {
    let repo = init_repo_with_origin();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    fs::write(slice_dir.join("01_first.md"), "slice").unwrap();
    commit_and_push(repo.path(), ["slices/01_first.md"], "add first slice");

    let client = Arc::new(ControlledClient::new());
    let start_count = Arc::clone(&client.start_count);
    let output = RecordingOutput::new();
    let lines = Arc::clone(&output.lines);
    let repo_root = repo.path().to_path_buf();
    let run_slice_dir = slice_dir.clone();
    let run_client = Arc::clone(&client);
    let local = LocalSet::new();

    local
        .run_until(async move {
            let handle = tokio::task::spawn_local(async move {
                let mut output = RecordingOutput { lines };
                run(
                    SupervisorOptions {
                        repo_root: repo_root.clone(),
                        slice_folder: run_slice_dir,
                        prompt_template: "prompt".to_string(),
                        schedule: ScheduleMode::Flat,
                    },
                    run_client.as_ref(),
                    &mut output,
                )
                .await
            });

            wait_for(|| start_count.load(Ordering::SeqCst) == 1).await;
            client.send_progress(ProgressPhase::Planning);
            client.send_progress(ProgressPhase::Planning);
            client.send_event(DebugEvent::ReviewPollingStarted);
            client.send_progress(ProgressPhase::WaitingForReview);
            wait_for(|| {
                output
                    .lines
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line == "event:01_first:review polling started")
            })
            .await;

            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });

            handle.await.unwrap().unwrap();

            let lines = output.lines.lock().unwrap().clone();
            assert!(lines.iter().any(|line| line == "phase:01_first:planning"));
            assert!(
                lines
                    .iter()
                    .any(|line| line == "event:01_first:review polling started")
            );
            assert!(
                lines
                    .iter()
                    .any(|line| line == "phase:01_first:waiting_for_review")
            );
            assert_eq!(
                lines
                    .iter()
                    .filter(|line| line.as_str() == "phase:01_first:planning")
                    .count(),
                1
            );
        })
        .await;

    let recovery_root = repo.path().join(".nsc").join("recovery");
    let run_dir = fs::read_dir(&recovery_root)
        .unwrap()
        .find_map(|entry| {
            let entry = entry.unwrap();
            entry.file_type().unwrap().is_dir().then_some(entry.path())
        })
        .unwrap();
    let recovery_file = fs::read_dir(run_dir)
        .unwrap()
        .find_map(|entry| {
            let entry = entry.unwrap();
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(".json")
                .then_some(entry.path())
        })
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(recovery_file).unwrap()).unwrap();

    assert_eq!(
        value.get("progress"),
        Some(&serde_json::json!("waiting_for_review"))
    );
    assert!(value.get("progress_updated_at").is_some());
    assert_eq!(
        value.get("status"),
        Some(&serde_json::json!("finished_prompt_owned"))
    );
}

#[tokio::test]
async fn wave_dirs_reject_local_only_slice_before_launch() {
    let repo = init_repo_with_origin();
    let root = repo.path().join("waves");
    fs::create_dir_all(root.join("batch02_first")).unwrap();
    fs::write(root.join("README.md"), "wave root").unwrap();
    fs::write(root.join("batch02_first").join("01_local_only.md"), "slice").unwrap();

    let client = ImmediateClient::new();
    let start_count = Arc::clone(&client.start_count);
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
    assert_eq!(start_count.load(Ordering::SeqCst), 0);
    let lines = output.lines.lock().unwrap().clone();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("wave-stopped:1/1:batch02_first:"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("slice file is missing from execution base"))
    );
}

#[tokio::test]
async fn later_internal_launch_failure_stops_the_run() {
    let repo = init_repo_with_origin();
    let slice_dir = repo.path().join("slices");
    fs::create_dir_all(&slice_dir).unwrap();
    for idx in 1..=4 {
        fs::write(
            slice_dir.join(format!("{idx:02}_slice.md")),
            format!("slice {idx}"),
        )
        .unwrap();
    }
    commit_and_push(
        repo.path(),
        [
            "slices/01_slice.md",
            "slices/02_slice.md",
            "slices/03_slice.md",
            "slices/04_slice.md",
        ],
        "add slices",
    );

    let client = Arc::new(ControlledClient::new());
    let start_count = Arc::clone(&client.start_count);
    let output = RecordingOutput::new();
    let lines = Arc::clone(&output.lines);
    let repo_root = repo.path().to_path_buf();
    let run_slice_dir = slice_dir.clone();
    let run_client = Arc::clone(&client);
    let local = LocalSet::new();

    local
        .run_until(async move {
            let handle = tokio::task::spawn_local(async move {
                let mut output = RecordingOutput { lines };
                run(
                    SupervisorOptions {
                        repo_root,
                        slice_folder: run_slice_dir,
                        prompt_template: "prompt".to_string(),
                        schedule: ScheduleMode::Flat,
                    },
                    run_client.as_ref(),
                    &mut output,
                )
                .await
            });

            wait_for(|| start_count.load(Ordering::SeqCst) == 3).await;
            let run_scope_root = repo.path().join(".nsc").join("worktrees");
            wait_for(|| {
                run_scope_root.is_dir()
                    && fs::read_dir(&run_scope_root)
                        .ok()
                        .and_then(|mut entries| entries.next())
                        .is_some()
            })
            .await;
            let run_scope = fs::read_dir(&run_scope_root)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let conflict_path = run_scope.join("04_slice");
            fs::create_dir_all(&conflict_path).unwrap();
            fs::write(conflict_path.join("CONFLICT.txt"), "conflict").unwrap();

            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });

            let result = handle.await.unwrap();
            assert!(result.is_err());
            let rendered = output.lines.lock().unwrap().clone();
            assert!(rendered.iter().any(|line| line.starts_with("run-stopped:")));
        })
        .await;
}

#[tokio::test]
async fn flat_mode_keeps_using_the_startup_base_sha_when_origin_main_moves() {
    let workspace = init_clone_with_external_origin();
    add_slices_in_source_repo(
        &workspace.source_repo,
        [
            "slices/01_slice.md",
            "slices/02_slice.md",
            "slices/03_slice.md",
            "slices/04_slice.md",
        ],
    );
    git_ok(
        &workspace.clone_repo,
        ["pull", "--ff-only", "origin", "main"],
    );
    let initial_base_sha = git_stdout(&workspace.clone_repo, ["rev-parse", "origin/main"]);

    let client = Arc::new(ControlledClient::new());
    let start_count = Arc::clone(&client.start_count);
    let output = RecordingOutput::new();
    let lines = Arc::clone(&output.lines);
    let repo_root = workspace.clone_repo.clone();
    let run_slice_dir = workspace.clone_repo.join("slices");
    let run_client = Arc::clone(&client);
    let local = LocalSet::new();

    local
        .run_until(async move {
            let handle = tokio::task::spawn_local(async move {
                let mut output = RecordingOutput { lines };
                run(
                    SupervisorOptions {
                        repo_root,
                        slice_folder: run_slice_dir,
                        prompt_template: "prompt".to_string(),
                        schedule: ScheduleMode::Flat,
                    },
                    run_client.as_ref(),
                    &mut output,
                )
                .await
            });

            wait_for(|| start_count.load(Ordering::SeqCst) == 3).await;
            advance_origin_readme(&workspace.source_repo, "origin moved");
            git_ok(
                &workspace.clone_repo,
                [
                    "fetch",
                    "origin",
                    "+refs/heads/main:refs/remotes/origin/main",
                ],
            );
            let refreshed_origin_sha =
                git_stdout(&workspace.clone_repo, ["rev-parse", "origin/main"]);
            assert_ne!(refreshed_origin_sha, initial_base_sha);

            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });
            wait_for(|| start_count.load(Ordering::SeqCst) == 4).await;
            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });
            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });
            client.complete_one(TerminalRunOutcome::FinishedPromptOwned {
                terminal_message: "done".to_string(),
            });

            handle.await.unwrap().unwrap();
            assert_eq!(start_count.load(Ordering::SeqCst), 4);

            let worktrees = client.worktree_paths.lock().unwrap().clone();
            let fourth_head = git_stdout(&worktrees[3], ["rev-parse", "HEAD"]);
            assert_eq!(fourth_head, initial_base_sha);
        })
        .await;
}

async fn wait_for(mut predicate: impl FnMut() -> bool) {
    for _ in 0..200 {
        if predicate() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("condition was not met in time");
}

fn init_repo_with_origin() -> TempDir {
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

struct ExternalOriginWorkspace {
    _temp: TempDir,
    source_repo: std::path::PathBuf,
    clone_repo: std::path::PathBuf,
}

fn init_clone_with_external_origin() -> ExternalOriginWorkspace {
    let temp = TempDir::new().unwrap();
    let origin_repo = temp.path().join("origin.git");
    let source_repo = temp.path().join("source");
    let clone_repo = temp.path().join("clone");

    fs::create_dir_all(&origin_repo).unwrap();
    git_ok(
        &origin_repo,
        ["init", "--bare", "--initial-branch=main", "."],
    );

    fs::create_dir_all(&source_repo).unwrap();
    git_ok(&source_repo, ["init", "--initial-branch=main"]);
    fs::write(source_repo.join("README.md"), "initial").unwrap();
    git_ok(&source_repo, ["add", "."]);
    git_ok(
        &source_repo,
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
    git_ok(
        &source_repo,
        ["remote", "add", "origin", origin_repo.to_str().unwrap()],
    );
    git_ok(&source_repo, ["push", "-u", "origin", "main"]);
    git_ok(
        temp.path(),
        [
            "clone",
            origin_repo.to_str().unwrap(),
            clone_repo.to_str().unwrap(),
        ],
    );

    ExternalOriginWorkspace {
        _temp: temp,
        source_repo,
        clone_repo,
    }
}

fn add_slices_in_source_repo<const N: usize>(repo_root: &Path, paths: [&str; N]) {
    for (index, path) in paths.into_iter().enumerate() {
        let full_path = repo_root.join(path);
        fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        fs::write(full_path, format!("slice {}", index + 1)).unwrap();
    }
    commit_and_push(repo_root, paths, "add slices");
}

fn advance_origin_readme(repo_root: &Path, contents: &str) {
    fs::write(repo_root.join("README.md"), contents).unwrap();
    commit_and_push(repo_root, ["README.md"], "advance origin");
}

fn commit_and_push<const N: usize>(repo_root: &Path, paths: [&str; N], message: &str) {
    for path in paths {
        git_ok(repo_root, ["add", path]);
    }
    git_ok(
        repo_root,
        [
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test User",
            "commit",
            "-m",
            message,
        ],
    );
    git_ok(repo_root, ["push", "origin", "main"]);
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
