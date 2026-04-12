use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::app_server::{AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome};
use crate::batch::{discover_new_candidates_on_rescan, discover_startup_candidates};
use crate::error::{NscError, Result};
use crate::output::{OutputEvent, OutputSink};
use crate::recovery::{RecoveryRecord, write_record};
use crate::slice::{MAX_PARALLEL_SLICES, SliceCandidate, SliceStatus};
use crate::worktree::{create_worktree, run_scope_now};

pub struct SupervisorOptions {
    pub repo_root: PathBuf,
    pub slice_folder: PathBuf,
    pub prompt_template: String,
}

struct RunningSlice {
    label: String,
    slice_file_path: PathBuf,
    worktree_path: PathBuf,
    thread_id: String,
    completion: tokio::sync::oneshot::Receiver<Result<TerminalRunOutcome>>,
}

pub async fn run(
    options: SupervisorOptions,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<()> {
    let run_scope = run_scope_now();
    let mut known_paths = HashSet::new();
    let mut pending = VecDeque::new();
    let startup_candidates = match discover_startup_candidates(&options.slice_folder) {
        Ok(candidates) => candidates,
        Err(err) => {
            output.emit(OutputEvent::FatalStartupFailure {
                message: &err.to_string(),
            });
            return Err(err);
        }
    };
    for candidate in startup_candidates {
        known_paths.insert(candidate.path.clone());
        pending.push_back(candidate);
    }

    let total_slices = pending.len();
    let initial_labels = pending
        .iter()
        .take(MAX_PARALLEL_SLICES)
        .map(|candidate| candidate.label.clone())
        .collect::<Vec<_>>();
    output.emit(OutputEvent::StartupSummary {
        total_slices,
        starting_labels: &initial_labels,
    });

    let mut running = Vec::new();
    launch_available(
        &options,
        &run_scope,
        client,
        output,
        &mut pending,
        &mut running,
        true,
    )
    .await?;

    while !running.is_empty() || !pending.is_empty() {
        if running.is_empty() {
            break;
        }
        let (index, result) = wait_for_one(&mut running).await?;
        let finished = running.swap_remove(index);
        handle_terminal_result(&options.repo_root, &run_scope, output, finished, result)?;

        let rescan = discover_new_candidates_on_rescan(&options.slice_folder, &known_paths);
        for warning in rescan.warnings {
            output.emit(OutputEvent::Warning { message: &warning });
        }
        for candidate in rescan.new_candidates {
            known_paths.insert(candidate.path.clone());
            output.emit(OutputEvent::NewSliceDiscovered {
                label: &candidate.label,
            });
            pending.push_back(candidate);
        }

        launch_available(
            &options,
            &run_scope,
            client,
            output,
            &mut pending,
            &mut running,
            false,
        )
        .await?;
    }

    Ok(())
}

async fn launch_available(
    options: &SupervisorOptions,
    run_scope: &str,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
    pending: &mut VecDeque<SliceCandidate>,
    running: &mut Vec<RunningSlice>,
    startup_phase: bool,
) -> Result<()> {
    while running.len() < MAX_PARALLEL_SLICES {
        let Some(candidate) = pending.pop_front() else {
            break;
        };

        if !startup_phase {
            output.emit(OutputEvent::SliceStarting {
                label: &candidate.label,
            });
        }

        let worktree_path = create_worktree(&options.repo_root, run_scope, &candidate.label)?;
        write_recovery(
            &options.repo_root,
            run_scope,
            &candidate.label,
            &candidate.path,
            &worktree_path,
            None,
            SliceStatus::Starting,
        )?;

        let request = RunSliceRequest {
            worktree_path: worktree_path.clone(),
            slice_file_path: candidate.path.clone(),
            prompt_template: options.prompt_template.clone(),
        };
        match client.start_run(request).await {
            Ok(StartedRun {
                thread_id,
                completion,
            }) => {
                write_recovery(
                    &options.repo_root,
                    run_scope,
                    &candidate.label,
                    &candidate.path,
                    &worktree_path,
                    Some(thread_id.clone()),
                    SliceStatus::Running,
                )?;
                running.push(RunningSlice {
                    label: candidate.label,
                    slice_file_path: candidate.path,
                    worktree_path,
                    thread_id,
                    completion,
                });
            }
            Err(err) => {
                write_recovery(
                    &options.repo_root,
                    run_scope,
                    &candidate.label,
                    &candidate.path,
                    &worktree_path,
                    None,
                    SliceStatus::AttentionNeeded,
                )?;
                output.emit(OutputEvent::SliceNeedsAttention {
                    label: &candidate.label,
                    thread_id: None,
                    worktree_path: &worktree_path,
                    reason: &err.to_string(),
                });
            }
        }
    }

    Ok(())
}

async fn wait_for_one(running: &mut [RunningSlice]) -> Result<(usize, Result<TerminalRunOutcome>)> {
    loop {
        for (index, slice) in running.iter_mut().enumerate() {
            match slice.completion.try_recv() {
                Ok(result) => return Ok((index, result)),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => continue,
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    return Ok((index, Err(NscError::ChannelClosed)));
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn handle_terminal_result(
    repo_root: &Path,
    run_scope: &str,
    output: &mut dyn OutputSink,
    finished: RunningSlice,
    result: Result<TerminalRunOutcome>,
) -> Result<()> {
    match result {
        Ok(TerminalRunOutcome::FinishedPromptOwned { terminal_message }) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::FinishedPromptOwned,
            )?;
            output.emit(OutputEvent::SliceFinishedNormally {
                label: &finished.label,
                terminal_message: &terminal_message,
            });
        }
        Ok(TerminalRunOutcome::AttentionNeeded { reason })
        | Err(NscError::AppServer(reason))
        | Err(NscError::Protocol(reason)) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::AttentionNeeded,
            )?;
            output.emit(OutputEvent::SliceNeedsAttention {
                label: &finished.label,
                thread_id: Some(&finished.thread_id),
                worktree_path: &finished.worktree_path,
                reason: &reason,
            });
        }
        Err(err) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::AttentionNeeded,
            )?;
            output.emit(OutputEvent::SliceNeedsAttention {
                label: &finished.label,
                thread_id: Some(&finished.thread_id),
                worktree_path: &finished.worktree_path,
                reason: &err.to_string(),
            });
        }
    }
    Ok(())
}

fn write_recovery(
    repo_root: &Path,
    run_scope: &str,
    label: &str,
    slice_file_path: &Path,
    worktree_path: &Path,
    thread_id: Option<String>,
    status: SliceStatus,
) -> Result<()> {
    write_record(
        repo_root,
        run_scope,
        label,
        &RecoveryRecord {
            slice_file_path: slice_file_path.display().to_string(),
            worktree_path: worktree_path.display().to_string(),
            thread_id,
            status,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use tempfile::TempDir;
    use tokio::sync::oneshot;

    use crate::app_server::{AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome};
    use crate::batch::discover_startup_candidates;
    use crate::error::Result;
    use crate::output::{OutputEvent, OutputSink};

    use super::{SupervisorOptions, run};

    struct RecordingOutput {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingOutput {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl OutputSink for RecordingOutput {
        fn emit(&mut self, event: OutputEvent<'_>) {
            let line = match event {
                OutputEvent::StartupSummary { total_slices, .. } => {
                    format!("startup:{total_slices}")
                }
                OutputEvent::SliceStarting { label } => format!("starting:{label}"),
                OutputEvent::SliceFinishedNormally { label, .. } => format!("finished:{label}"),
                OutputEvent::SliceNeedsAttention { label, .. } => format!("attention:{label}"),
                OutputEvent::NewSliceDiscovered { label } => format!("new:{label}"),
                OutputEvent::Warning { message } => format!("warning:{message}"),
                OutputEvent::FatalStartupFailure { message } => format!("fatal:{message}"),
            };
            self.events.lock().unwrap().push(line);
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
                thread_id: "thread-1".to_string(),
                completion: rx,
            })
        }
    }

    #[tokio::test]
    async fn startup_validation_rejects_empty_slice_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("01_good.md"), "content").unwrap();
        fs::write(temp.path().join("02_empty.md"), "").unwrap();

        let err = discover_startup_candidates(temp.path()).unwrap_err();
        assert!(err.to_string().contains("slice file is empty"));
    }

    #[tokio::test]
    async fn supervisor_processes_more_than_three_slices() {
        let repo = init_git_repo();
        let slice_dir = repo.path().join("slices");
        fs::create_dir_all(&slice_dir).unwrap();
        for idx in 1..=5 {
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
            5
        ]);
        let mut output = RecordingOutput::new();

        let options = SupervisorOptions {
            repo_root: repo.path().to_path_buf(),
            slice_folder: slice_dir,
            prompt_template: "prompt".to_string(),
        };

        run(options, &client, &mut output).await.unwrap();
        let lines = output.events.lock().unwrap().clone();
        assert!(lines.iter().any(|line| line == "startup:5"));
        assert!(
            lines
                .iter()
                .filter(|line| line.starts_with("finished:"))
                .count()
                == 5
        );
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
}
