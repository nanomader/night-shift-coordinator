use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::app_server::{AppServerClient, RunSliceRequest, StartedRun, TerminalRunOutcome};
use crate::batch::{discover_new_candidates_on_rescan, discover_startup_candidates};
use crate::cli::ScheduleMode;
use crate::error::{NscError, Result};
use crate::output::{OutputEvent, OutputSink};
use crate::recovery::{RecoveryRecord, write_record};
use crate::slice::{MAX_PARALLEL_SLICES, SliceCandidate, SliceStatus};
use crate::terminal_result::{TerminalResultKind, parse_terminal_result_markers};
use crate::wave_dirs::discover_wave_dirs;
use crate::worktree::{
    create_worktree, origin_main_contains_commit, prepare_default_base_ref, run_scope_now,
};

pub struct SupervisorOptions {
    pub repo_root: PathBuf,
    pub slice_folder: PathBuf,
    pub prompt_template: String,
    pub schedule: ScheduleMode,
}

#[derive(Debug, Clone)]
struct QueuedSlice {
    slice_file_path: PathBuf,
    output_label: String,
    internal_label: String,
}

struct RunningSlice {
    output_label: String,
    internal_label: String,
    slice_file_path: PathBuf,
    worktree_path: PathBuf,
    thread_id: String,
    started_at: Instant,
    last_progress_log_at: Instant,
    completion: tokio::sync::oneshot::Receiver<Result<TerminalRunOutcome>>,
}

#[derive(Debug, Clone)]
struct BatchRunSummary {
    completed_slices: Vec<CompletedSlice>,
}

#[derive(Debug, Clone)]
struct CompletedSlice {
    output_label: String,
    outcome: CompletedSliceOutcome,
}

#[derive(Debug, Clone)]
enum CompletedSliceOutcome {
    FinishedPromptOwned { terminal_message: String },
    AttentionNeeded { reason: String },
}

const PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

struct BatchExecutionContext<'a> {
    repo_root: &'a Path,
    run_scope: &'a str,
    prompt_template: &'a str,
    base_ref: &'a str,
}

pub async fn run(
    options: SupervisorOptions,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<()> {
    let run_scope = run_scope_now();
    match options.schedule {
        ScheduleMode::Flat => {
            let base_ref = match prepare_default_base_ref(&options.repo_root) {
                Ok(base_ref) => base_ref,
                Err(err) => {
                    output.emit(OutputEvent::FatalStartupFailure {
                        message: &err.to_string(),
                    });
                    return Err(err);
                }
            };
            run_flat_batch(
                &options,
                &run_scope,
                &base_ref,
                None,
                &options.slice_folder,
                client,
                output,
            )
            .await
            .map(|_| ())
            .inspect_err(|err| {
                output.emit(OutputEvent::FatalStartupFailure {
                    message: &err.to_string(),
                });
            })
        }
        ScheduleMode::WaveDirs => run_wave_dirs(&options, &run_scope, client, output).await,
    }
}

async fn run_wave_dirs(
    options: &SupervisorOptions,
    run_scope: &str,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<()> {
    let wave_dirs = match discover_wave_dirs(&options.slice_folder) {
        Ok(wave_dirs) => wave_dirs,
        Err(err) => {
            output.emit(OutputEvent::FatalStartupFailure {
                message: &err.to_string(),
            });
            return Err(err);
        }
    };

    output.emit(OutputEvent::WaveRootSummary {
        total_wave_dirs: wave_dirs.len(),
    });

    let mut next_base_ref: Option<String> = None;
    for (index, wave_dir) in wave_dirs.iter().enumerate() {
        output.emit(OutputEvent::WaveDirStarting {
            index: index + 1,
            total: wave_dirs.len(),
            name: &wave_dir.name,
        });

        let base_ref = match next_base_ref.take() {
            Some(base_ref) => base_ref,
            None => match prepare_default_base_ref(&options.repo_root) {
                Ok(base_ref) => base_ref,
                Err(err) => {
                    output.emit(OutputEvent::WaveDirStopped {
                        index: index + 1,
                        total: wave_dirs.len(),
                        name: &wave_dir.name,
                        reason: &err.to_string(),
                    });
                    return Err(err);
                }
            },
        };

        let summary = match run_flat_batch(
            options,
            run_scope,
            &base_ref,
            Some(&wave_dir.name),
            &wave_dir.path,
            client,
            output,
        )
        .await
        {
            Ok(summary) => summary,
            Err(err) => {
                output.emit(OutputEvent::WaveDirStopped {
                    index: index + 1,
                    total: wave_dirs.len(),
                    name: &wave_dir.name,
                    reason: &err.to_string(),
                });
                return Err(err);
            }
        };

        match verify_wave_dir_unlock(&options.repo_root, &wave_dir.name, &summary) {
            Ok(refreshed_base_ref) => {
                output.emit(OutputEvent::WaveDirFinished {
                    index: index + 1,
                    total: wave_dirs.len(),
                    name: &wave_dir.name,
                    result: TerminalResultKind::MergedSuccess.as_str(),
                });
                next_base_ref = Some(refreshed_base_ref);
            }
            Err(err) => {
                output.emit(OutputEvent::WaveDirStopped {
                    index: index + 1,
                    total: wave_dirs.len(),
                    name: &wave_dir.name,
                    reason: &err.to_string(),
                });
                return Err(err);
            }
        }
    }

    Ok(())
}

async fn run_flat_batch(
    options: &SupervisorOptions,
    run_scope: &str,
    base_ref: &str,
    label_scope: Option<&str>,
    batch_dir: &Path,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<BatchRunSummary> {
    let mut known_paths = HashSet::new();
    let mut pending = VecDeque::new();
    let startup_candidates = discover_startup_candidates(batch_dir)?;
    for candidate in startup_candidates {
        known_paths.insert(candidate.path.clone());
        pending.push_back(queue_slice(candidate, label_scope));
    }

    let total_slices = pending.len();
    let initial_labels = pending
        .iter()
        .take(MAX_PARALLEL_SLICES)
        .map(|candidate| candidate.output_label.clone())
        .collect::<Vec<_>>();
    output.emit(OutputEvent::StartupSummary {
        total_slices,
        starting_labels: &initial_labels,
    });

    let context = BatchExecutionContext {
        repo_root: &options.repo_root,
        run_scope,
        prompt_template: &options.prompt_template,
        base_ref,
    };
    let mut running = Vec::new();
    let mut completed_slices = Vec::new();
    launch_available(
        &context,
        client,
        output,
        &mut pending,
        &mut running,
        &mut completed_slices,
    )
    .await?;

    while !running.is_empty() || !pending.is_empty() {
        if running.is_empty() {
            break;
        }
        let (index, result) = wait_for_one(&mut running, output).await?;
        let finished = running.swap_remove(index);
        completed_slices.push(handle_terminal_result(
            &options.repo_root,
            run_scope,
            output,
            finished,
            result,
        )?);

        let rescan = discover_new_candidates_on_rescan(batch_dir, &known_paths);
        for warning in rescan.warnings {
            output.emit(OutputEvent::Warning { message: &warning });
        }
        for candidate in rescan.new_candidates {
            known_paths.insert(candidate.path.clone());
            let queued = queue_slice(candidate, label_scope);
            output.emit(OutputEvent::NewSliceDiscovered {
                label: &queued.output_label,
            });
            pending.push_back(queued);
        }

        launch_available(
            &context,
            client,
            output,
            &mut pending,
            &mut running,
            &mut completed_slices,
        )
        .await?;
    }

    Ok(BatchRunSummary { completed_slices })
}

fn queue_slice(candidate: SliceCandidate, label_scope: Option<&str>) -> QueuedSlice {
    let internal_label = match label_scope {
        Some(scope) => format!("{scope}__{}", candidate.label),
        None => candidate.label.clone(),
    };
    QueuedSlice {
        slice_file_path: candidate.path,
        output_label: candidate.label,
        internal_label,
    }
}

async fn launch_available(
    context: &BatchExecutionContext<'_>,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
    pending: &mut VecDeque<QueuedSlice>,
    running: &mut Vec<RunningSlice>,
    completed_slices: &mut Vec<CompletedSlice>,
) -> Result<()> {
    while running.len() < MAX_PARALLEL_SLICES {
        let Some(candidate) = pending.pop_front() else {
            break;
        };

        output.emit(OutputEvent::SliceStarting {
            label: &candidate.output_label,
        });

        let worktree_path = create_worktree(
            context.repo_root,
            context.run_scope,
            &candidate.internal_label,
            context.base_ref,
        )?;
        write_recovery(
            context.repo_root,
            context.run_scope,
            &candidate.internal_label,
            &candidate.slice_file_path,
            &worktree_path,
            None,
            SliceStatus::Starting,
        )?;

        let request = RunSliceRequest {
            worktree_path: worktree_path.clone(),
            slice_repo_relative_path: repo_relative_slice_path(
                context.repo_root,
                &candidate.slice_file_path,
            )?,
            prompt_template: context.prompt_template.to_string(),
        };
        match client.start_run(request).await {
            Ok(StartedRun {
                thread_id,
                completion,
            }) => {
                let now = Instant::now();
                write_recovery(
                    context.repo_root,
                    context.run_scope,
                    &candidate.internal_label,
                    &candidate.slice_file_path,
                    &worktree_path,
                    Some(thread_id.clone()),
                    SliceStatus::Running,
                )?;
                output.emit(OutputEvent::SliceRunning {
                    label: &candidate.output_label,
                    thread_id: &thread_id,
                    worktree_path: &worktree_path,
                });
                running.push(RunningSlice {
                    output_label: candidate.output_label,
                    internal_label: candidate.internal_label,
                    slice_file_path: candidate.slice_file_path,
                    worktree_path,
                    thread_id,
                    started_at: now,
                    last_progress_log_at: now,
                    completion,
                });
            }
            Err(err) => {
                write_recovery(
                    context.repo_root,
                    context.run_scope,
                    &candidate.internal_label,
                    &candidate.slice_file_path,
                    &worktree_path,
                    None,
                    SliceStatus::AttentionNeeded,
                )?;
                output.emit(OutputEvent::SliceNeedsAttention {
                    label: &candidate.output_label,
                    thread_id: None,
                    worktree_path: &worktree_path,
                    reason: &err.to_string(),
                });
                completed_slices.push(CompletedSlice {
                    output_label: candidate.output_label,
                    outcome: CompletedSliceOutcome::AttentionNeeded {
                        reason: err.to_string(),
                    },
                });
            }
        }
    }

    Ok(())
}

fn repo_relative_slice_path(repo_root: &Path, slice_path: &Path) -> Result<PathBuf> {
    slice_path
        .strip_prefix(repo_root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            NscError::Protocol(format!(
                "slice path is outside repo root: repo={}, slice={}",
                repo_root.display(),
                slice_path.display()
            ))
        })
}

async fn wait_for_one(
    running: &mut [RunningSlice],
    output: &mut dyn OutputSink,
) -> Result<(usize, Result<TerminalRunOutcome>)> {
    loop {
        for (index, slice) in running.iter_mut().enumerate() {
            if slice.last_progress_log_at.elapsed() >= PROGRESS_HEARTBEAT_INTERVAL {
                output.emit(OutputEvent::SliceStillRunning {
                    label: &slice.output_label,
                    elapsed_seconds: slice.started_at.elapsed().as_secs(),
                });
                slice.last_progress_log_at = Instant::now();
            }
            match slice.completion.try_recv() {
                Ok(result) => return Ok((index, result)),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => continue,
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    return Ok((index, Err(NscError::ChannelClosed)));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn handle_terminal_result(
    repo_root: &Path,
    run_scope: &str,
    output: &mut dyn OutputSink,
    finished: RunningSlice,
    result: Result<TerminalRunOutcome>,
) -> Result<CompletedSlice> {
    match result {
        Ok(TerminalRunOutcome::FinishedPromptOwned { terminal_message }) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.internal_label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::FinishedPromptOwned,
            )?;
            output.emit(OutputEvent::SliceFinishedNormally {
                label: &finished.output_label,
                terminal_message: &terminal_message,
            });
            Ok(CompletedSlice {
                output_label: finished.output_label,
                outcome: CompletedSliceOutcome::FinishedPromptOwned { terminal_message },
            })
        }
        Ok(TerminalRunOutcome::AttentionNeeded { reason })
        | Err(NscError::AppServer(reason))
        | Err(NscError::Protocol(reason)) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.internal_label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::AttentionNeeded,
            )?;
            output.emit(OutputEvent::SliceNeedsAttention {
                label: &finished.output_label,
                thread_id: Some(&finished.thread_id),
                worktree_path: &finished.worktree_path,
                reason: &reason,
            });
            Ok(CompletedSlice {
                output_label: finished.output_label,
                outcome: CompletedSliceOutcome::AttentionNeeded { reason },
            })
        }
        Err(err) => {
            write_recovery(
                repo_root,
                run_scope,
                &finished.internal_label,
                &finished.slice_file_path,
                &finished.worktree_path,
                Some(finished.thread_id.clone()),
                SliceStatus::AttentionNeeded,
            )?;
            output.emit(OutputEvent::SliceNeedsAttention {
                label: &finished.output_label,
                thread_id: Some(&finished.thread_id),
                worktree_path: &finished.worktree_path,
                reason: &err.to_string(),
            });
            Ok(CompletedSlice {
                output_label: finished.output_label,
                outcome: CompletedSliceOutcome::AttentionNeeded {
                    reason: err.to_string(),
                },
            })
        }
    }
}

fn verify_wave_dir_unlock(
    repo_root: &Path,
    wave_dir_name: &str,
    summary: &BatchRunSummary,
) -> Result<String> {
    let refreshed_base_ref = prepare_default_base_ref(repo_root)?;

    for slice in &summary.completed_slices {
        match &slice.outcome {
            CompletedSliceOutcome::AttentionNeeded { reason } => {
                return Err(NscError::WaveDirUnlockFailed {
                    wave_dir: wave_dir_name.to_string(),
                    reason: format!(
                        "slice {} needs attention before next wave-dir: {reason}",
                        slice.output_label
                    ),
                });
            }
            CompletedSliceOutcome::FinishedPromptOwned { terminal_message } => {
                let Some(markers) = parse_terminal_result_markers(terminal_message)? else {
                    return Err(NscError::WaveDirUnlockFailed {
                        wave_dir: wave_dir_name.to_string(),
                        reason: format!(
                            "slice {} finished without NSC_RESULT markers",
                            slice.output_label
                        ),
                    });
                };
                if markers.result != TerminalResultKind::MergedSuccess {
                    return Err(NscError::WaveDirUnlockFailed {
                        wave_dir: wave_dir_name.to_string(),
                        reason: format!(
                            "slice {} reported {}",
                            slice.output_label,
                            markers.result.as_str()
                        ),
                    });
                }
                let merged_main_sha = markers.merged_main_sha.as_deref().ok_or_else(|| {
                    NscError::WaveDirUnlockFailed {
                        wave_dir: wave_dir_name.to_string(),
                        reason: format!(
                            "slice {} is missing NSC_MERGED_MAIN_SHA",
                            slice.output_label
                        ),
                    }
                })?;
                if markers.pr_number.is_none() {
                    return Err(NscError::WaveDirUnlockFailed {
                        wave_dir: wave_dir_name.to_string(),
                        reason: format!("slice {} is missing NSC_PR", slice.output_label),
                    });
                }
                if !origin_main_contains_commit(repo_root, merged_main_sha)? {
                    return Err(NscError::WaveDirUnlockFailed {
                        wave_dir: wave_dir_name.to_string(),
                        reason: format!(
                            "slice {} reported merged SHA {} but fetched origin/main does not contain it",
                            slice.output_label, merged_main_sha
                        ),
                    });
                }
            }
        }
    }

    Ok(refreshed_base_ref)
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
