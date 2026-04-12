use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NscError {
    #[error("path is not a directory: {0}")]
    InvalidDirectory(PathBuf),
    #[error("no slice candidates found in {0}")]
    NoSliceCandidates(PathBuf),
    #[error("slice file is empty: {0}")]
    EmptySliceFile(PathBuf),
    #[error("path is not readable: {path}: {source}")]
    PathIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("prompt template is unreadable: {path}: {source}")]
    PromptTemplateRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("slice folder is outside the repo root: repo={repo_root}, slice_folder={slice_folder}")]
    SliceFolderOutsideRepo {
        repo_root: PathBuf,
        slice_folder: PathBuf,
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
