use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NscError {
    #[error("path is not a directory: {0}")]
    InvalidDirectory(PathBuf),
    #[error("no slice candidates found in {0}")]
    NoSliceCandidates(PathBuf),
    #[error("no wave directories found in {0}")]
    NoWaveDirs(PathBuf),
    #[error("no wave roots found in {0}")]
    NoWaveRoots(PathBuf),
    #[error("slice file is empty: {0}")]
    EmptySliceFile(PathBuf),
    #[error("path is not readable: {path}: {source}")]
    PathIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("unexpected entry in wave root {root}: {entry}")]
    InvalidWaveRootEntry { root: PathBuf, entry: String },
    #[error("unexpected entry in program root {root}: {entry}")]
    InvalidProgramRootEntry { root: PathBuf, entry: String },
    #[error("invalid wave directory name in {root}: {entry}")]
    InvalidWaveDirName { root: PathBuf, entry: String },
    #[error("invalid wave root name in {root}: {entry}")]
    InvalidWaveRootName { root: PathBuf, entry: String },
    #[error("duplicate wave directory batch id {batch_id:02} in {root}")]
    DuplicateWaveDirBatchId { root: PathBuf, batch_id: u32 },
    #[error("duplicate wave root id {wave_id:02} in {root}")]
    DuplicateWaveRootId { root: PathBuf, wave_id: u32 },
    #[error(
        "wave directory numbering is not contiguous in {root}: expected batch{expected:02}_..., found batch{actual:02}_..."
    )]
    WaveDirGap {
        root: PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error(
        "wave root numbering is not contiguous in {root}: expected wave{expected:02}_..., found wave{actual:02}_..."
    )]
    WaveRootGap {
        root: PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error("invalid terminal result markers: {0}")]
    InvalidTerminalResult(String),
    #[error("slice file is missing from execution base {base_ref}: {slice_path}")]
    SliceMissingOnExecutionBase {
        slice_path: PathBuf,
        base_ref: String,
    },
    #[error("wave-dir unlock failed for {wave_dir}: {reason}")]
    WaveDirUnlockFailed { wave_dir: String, reason: String },
    #[error("prompt template is unreadable: {path}: {source}")]
    PromptTemplateRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("target path is outside the repo root: repo={repo_root}, target_path={target_path}")]
    TargetPathOutsideRepo {
        repo_root: PathBuf,
        target_path: PathBuf,
    },
    #[error("not running from repo root: expected {expected}, got {actual}")]
    NotRepoRoot { expected: PathBuf, actual: PathBuf },
    #[error("git command failed: {0}")]
    Git(String),
    #[error("codex app-server failed: {0}")]
    AppServer(String),
    #[error("codex app-server protocol error: {0}")]
    Protocol(String),
    #[error("channel closed before slice outcome was received")]
    ChannelClosed,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, NscError>;
