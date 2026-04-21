use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::app_server::{
    AppServerClient, RunSliceRequest, RunUpdate, StartedRun, TerminalRunOutcome,
};
use crate::batch::discover_startup_candidates;
use crate::cli::ScheduleMode;
use crate::error::{NscError, Result};
use crate::output::{OutputEvent, OutputSink};
use crate::recovery::{RecoveryRecord, write_record};
use crate::slice::{MAX_PARALLEL_SLICES, SliceCandidate, SliceStatus};
use crate::terminal_result::{
    ProgressPhase, TerminalResultKind, TerminalResultMarkers, parse_terminal_result_markers,
};
use crate::wave_dirs::{WaveDir, discover_wave_dirs};
use crate::wave_roots::discover_wave_roots;
use crate::worktree::{
    base_ref_contains_repo_relative_path, create_worktree, origin_main_contains_commit,
    prepare_default_base_ref, run_scope_now,
};

pub struct SupervisorOptions {
    pub repo_root: PathBuf,
    pub target_root: PathBuf,
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
    latest_progress: Option<ProgressPhase>,
    latest_progress_updated_at: Option<String>,
    updates: tokio::sync::mpsc::UnboundedReceiver<RunUpdate>,
    completion: tokio::sync::oneshot::Receiver<Result<TerminalRunOutcome>>,
}

#[derive(Debug, Clone)]
struct RecoveryState {
    thread_id: Option<String>,
    status: SliceStatus,
    progress: Option<ProgressPhase>,
    progress_updated_at: Option<String>,
    terminal_markers: Option<TerminalResultMarkers>,
    completed_at: Option<String>,
}

#[derive(Debug, Clone)]
struct FlatBatchRunSummary {
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

struct BatchExecutionContext<'a> {
    repo_root: &'a Path,
    run_scope: &'a str,
    prompt_template: &'a str,
    base_ref: &'a str,
}

#[derive(Debug)]
enum BatchRunFailure {
    BeforeLaunch(NscError),
    AfterLaunch(NscError),
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
                &options.target_root,
                client,
                output,
            )
            .await
            .map(|_| ())
            .map_err(|failure| match failure {
                BatchRunFailure::BeforeLaunch(err) => {
                    output.emit(OutputEvent::FatalStartupFailure {
                        message: &err.to_string(),
                    });
                    err
                }
                BatchRunFailure::AfterLaunch(err) => {
                    output.emit(OutputEvent::RunStopped {
                        message: &err.to_string(),
                    });
                    err
                }
            })
        }
        ScheduleMode::WaveDirs => {
            let wave_dirs = match discover_wave_dirs(&options.target_root) {
                Ok(wave_dirs) => wave_dirs,
                Err(err) => {
                    output.emit(OutputEvent::FatalStartupFailure {
                        message: &err.to_string(),
                    });
                    return Err(err);
                }
            };
            run_wave_root(&options, &run_scope, None, wave_dirs, client, output).await
        }
        ScheduleMode::WaveRoots => run_program_root(&options, &run_scope, client, output).await,
    }
}

async fn run_program_root(
    options: &SupervisorOptions,
    run_scope: &str,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<()> {
    let wave_roots = match discover_wave_roots(&options.target_root) {
        Ok(wave_roots) => wave_roots,
        Err(err) => {
            output.emit(OutputEvent::FatalStartupFailure {
                message: &err.to_string(),
            });
            return Err(err);
        }
    };

    output.emit(OutputEvent::ProgramRootSummary {
        total_wave_roots: wave_roots.len(),
    });

    for (index, wave_root) in wave_roots.iter().enumerate() {
        output.emit(OutputEvent::WaveRootStarting {
            index: index + 1,
            total: wave_roots.len(),
            name: &wave_root.name,
        });

        let wave_dirs = match discover_wave_dirs(&wave_root.path) {
            Ok(wave_dirs) => wave_dirs,
            Err(err) => {
                output.emit(OutputEvent::WaveRootStopped {
                    index: index + 1,
                    total: wave_roots.len(),
                    name: &wave_root.name,
                    reason: &err.to_string(),
                });
                return Err(err);
            }
        };

        match run_wave_root(
            options,
            run_scope,
            Some(&wave_root.name),
            wave_dirs,
            client,
            output,
        )
        .await
        {
            Ok(()) => {
                output.emit(OutputEvent::WaveRootFinished {
                    index: index + 1,
                    total: wave_roots.len(),
                    name: &wave_root.name,
                    result: TerminalResultKind::MergedSuccess.as_str(),
                });
            }
            Err(err) => {
                output.emit(OutputEvent::WaveRootStopped {
                    index: index + 1,
                    total: wave_roots.len(),
                    name: &wave_root.name,
                    reason: &err.to_string(),
                });
                return Err(err);
            }
        }
    }

    Ok(())
}

async fn run_wave_root(
    options: &SupervisorOptions,
    run_scope: &str,
    root_label_scope: Option<&str>,
    wave_dirs: Vec<WaveDir>,
    client: &dyn AppServerClient,
    output: &mut dyn OutputSink,
) -> Result<()> {
    output.emit(OutputEvent::WaveDirRootSummary {
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

        let label_scope = wave_dir_label_scope(root_label_scope, &wave_dir.name);
        let summary = match run_flat_batch(
            options,
            run_scope,
            &base_ref,
            Some(label_scope.as_str()),
            &wave_dir.path,
            client,
            output,
        )
        .await
        {
            Ok(summary) => summary,
            Err(failure) => {
                let err = match failure {
                    BatchRunFailure::BeforeLaunch(err) | BatchRunFailure::AfterLaunch(err) => err,
                };
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
) -> std::result::Result<FlatBatchRunSummary, BatchRunFailure> {
    let mut pending = VecDeque::new();
    let startup_candidates =
        discover_startup_candidates(batch_dir).map_err(BatchRunFailure::BeforeLaunch)?;
    validate_candidates_against_base(&options.repo_root, base_ref, &startup_candidates)
        .map_err(BatchRunFailure::BeforeLaunch)?;
    for candidate in startup_candidates {
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
    .await
    .map_err(|err| map_batch_run_failure(err, &running, &completed_slices))?;

    while !running.is_empty() || !pending.is_empty() {
        if running.is_empty() {
            break;
        }
        let (index, result) = wait_for_one(&options.repo_root, run_scope, &mut running, output)
            .await
            .map_err(BatchRunFailure::AfterLaunch)?;
        let finished = running.swap_remove(index);
        completed_slices.push(
            handle_terminal_result(&options.repo_root, run_scope, output, finished, result)
                .map_err(BatchRunFailure::AfterLaunch)?,
        );

        launch_available(
            &context,
            client,
            output,
            &mut pending,
            &mut running,
            &mut completed_slices,
        )
        .await
        .map_err(|err| map_batch_run_failure(err, &running, &completed_slices))?;
    }

    Ok(FlatBatchRunSummary { completed_slices })
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
            RecoveryState {
                thread_id: None,
                status: SliceStatus::Starting,
                progress: None,
                progress_updated_at: None,
                terminal_markers: None,
                completed_at: None,
            },
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
                updates,
                completion,
            }) => {
                write_recovery(
                    context.repo_root,
                    context.run_scope,
                    &candidate.internal_label,
                    &candidate.slice_file_path,
                    &worktree_path,
                    RecoveryState {
                        thread_id: Some(thread_id.clone()),
                        status: SliceStatus::Running,
                        progress: None,
                        progress_updated_at: None,
                        terminal_markers: None,
                        completed_at: None,
                    },
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
                    latest_progress: None,
                    latest_progress_updated_at: None,
                    updates,
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
                    RecoveryState {
                        thread_id: None,
                        status: SliceStatus::AttentionNeeded,
                        progress: None,
                        progress_updated_at: None,
                        terminal_markers: None,
                        completed_at: None,
                    },
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

fn validate_candidates_against_base(
    repo_root: &Path,
    base_ref: &str,
    candidates: &[SliceCandidate],
) -> Result<()> {
    for candidate in candidates {
        let repo_relative_path = repo_relative_slice_path(repo_root, &candidate.path)?;
        if !base_ref_contains_repo_relative_path(repo_root, base_ref, &repo_relative_path)? {
            return Err(NscError::SliceMissingOnExecutionBase {
                slice_path: repo_relative_path,
                base_ref: base_ref.to_string(),
            });
        }
    }
    Ok(())
}

fn map_batch_run_failure(
    err: NscError,
    running: &[RunningSlice],
    completed_slices: &[CompletedSlice],
) -> BatchRunFailure {
    if running.is_empty() && completed_slices.is_empty() {
        BatchRunFailure::BeforeLaunch(err)
    } else {
        BatchRunFailure::AfterLaunch(err)
    }
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
    repo_root: &Path,
    run_scope: &str,
    running: &mut [RunningSlice],
    output: &mut dyn OutputSink,
) -> Result<(usize, Result<TerminalRunOutcome>)> {
    loop {
        for (index, slice) in running.iter_mut().enumerate() {
            while let Ok(update) = slice.updates.try_recv() {
                match update {
                    RunUpdate::Progress(progress) => {
                        if slice.latest_progress == Some(progress) {
                            continue;
                        }
                        slice.latest_progress = Some(progress);
                        slice.latest_progress_updated_at = Some(progress_updated_at_now()?);
                        write_recovery(
                            repo_root,
                            run_scope,
                            &slice.internal_label,
                            &slice.slice_file_path,
                            &slice.worktree_path,
                            RecoveryState {
                                thread_id: Some(slice.thread_id.clone()),
                                status: SliceStatus::Running,
                                progress: slice.latest_progress,
                                progress_updated_at: slice.latest_progress_updated_at.clone(),
                                terminal_markers: None,
                                completed_at: None,
                            },
                        )?;
                        output.emit(OutputEvent::SliceProgress {
                            label: &slice.output_label,
                            phase: progress,
                        });
                    }
                    RunUpdate::Event(event) => {
                        slice.latest_progress_updated_at = Some(progress_updated_at_now()?);
                        write_recovery(
                            repo_root,
                            run_scope,
                            &slice.internal_label,
                            &slice.slice_file_path,
                            &slice.worktree_path,
                            RecoveryState {
                                thread_id: Some(slice.thread_id.clone()),
                                status: SliceStatus::Running,
                                progress: slice.latest_progress,
                                progress_updated_at: slice.latest_progress_updated_at.clone(),
                                terminal_markers: None,
                                completed_at: None,
                            },
                        )?;
                        output.emit(OutputEvent::SliceWorkflowEvent {
                            label: &slice.output_label,
                            event,
                        });
                    }
                }
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
            let terminal_markers = match parse_terminal_result_markers(&terminal_message) {
                Ok(markers) => markers,
                Err(NscError::InvalidTerminalResult(reason)) => {
                    let reason = format!("invalid terminal result markers: {reason}");
                    write_recovery(
                        repo_root,
                        run_scope,
                        &finished.internal_label,
                        &finished.slice_file_path,
                        &finished.worktree_path,
                        RecoveryState {
                            thread_id: Some(finished.thread_id.clone()),
                            status: SliceStatus::AttentionNeeded,
                            progress: finished.latest_progress,
                            progress_updated_at: finished.latest_progress_updated_at,
                            terminal_markers: None,
                            completed_at: Some(completed_at_now()?),
                        },
                    )?;
                    output.emit(OutputEvent::SliceNeedsAttention {
                        label: &finished.output_label,
                        thread_id: Some(&finished.thread_id),
                        worktree_path: &finished.worktree_path,
                        reason: &reason,
                    });
                    return Ok(CompletedSlice {
                        output_label: finished.output_label,
                        outcome: CompletedSliceOutcome::AttentionNeeded { reason },
                    });
                }
                Err(err) => return Err(err),
            };
            write_recovery(
                repo_root,
                run_scope,
                &finished.internal_label,
                &finished.slice_file_path,
                &finished.worktree_path,
                RecoveryState {
                    thread_id: Some(finished.thread_id.clone()),
                    status: SliceStatus::FinishedPromptOwned,
                    progress: finished.latest_progress,
                    progress_updated_at: finished.latest_progress_updated_at,
                    terminal_markers: terminal_markers.clone(),
                    completed_at: Some(completed_at_now()?),
                },
            )?;
            output.emit(OutputEvent::SliceFinishedNormally {
                label: &finished.output_label,
                terminal_message: &terminal_message,
                terminal_markers: terminal_markers.as_ref(),
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
                RecoveryState {
                    thread_id: Some(finished.thread_id.clone()),
                    status: SliceStatus::AttentionNeeded,
                    progress: finished.latest_progress,
                    progress_updated_at: finished.latest_progress_updated_at,
                    terminal_markers: None,
                    completed_at: None,
                },
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
                RecoveryState {
                    thread_id: Some(finished.thread_id.clone()),
                    status: SliceStatus::AttentionNeeded,
                    progress: finished.latest_progress,
                    progress_updated_at: finished.latest_progress_updated_at,
                    terminal_markers: None,
                    completed_at: None,
                },
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
    summary: &FlatBatchRunSummary,
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

fn wave_dir_label_scope(root_label_scope: Option<&str>, wave_dir_name: &str) -> String {
    match root_label_scope {
        Some(root_label) => format!("{root_label}__{wave_dir_name}"),
        None => wave_dir_name.to_string(),
    }
}

fn write_recovery(
    repo_root: &Path,
    run_scope: &str,
    label: &str,
    slice_file_path: &Path,
    worktree_path: &Path,
    state: RecoveryState,
) -> Result<()> {
    let result = state
        .terminal_markers
        .as_ref()
        .map(|markers| markers.result);
    let pr_number = state
        .terminal_markers
        .as_ref()
        .and_then(|markers| markers.pr_number);
    let merged_main_sha = state
        .terminal_markers
        .as_ref()
        .and_then(|markers| markers.merged_main_sha.clone());
    write_record(
        repo_root,
        run_scope,
        label,
        &RecoveryRecord {
            slice_file_path: slice_file_path.display().to_string(),
            worktree_path: worktree_path.display().to_string(),
            thread_id: state.thread_id,
            status: state.status,
            progress: state.progress,
            progress_updated_at: state.progress_updated_at,
            result,
            pr_number,
            merged_main_sha,
            completed_at: state.completed_at,
        },
    )?;
    Ok(())
}

fn progress_updated_at_now() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| NscError::Protocol(format!("failed to format progress timestamp: {err}")))
}

fn completed_at_now() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| NscError::Protocol(format!("failed to format completion timestamp: {err}")))
}
