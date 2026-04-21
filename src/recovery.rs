use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::slice::SliceStatus;
use crate::terminal_result::{ProgressPhase, TerminalResultKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryRecord {
    pub slice_file_path: String,
    pub worktree_path: String,
    pub thread_id: Option<String>,
    pub status: SliceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<TerminalResultKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged_main_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

pub fn recovery_path(repo_root: &Path, run_scope: &str, label: &str) -> PathBuf {
    repo_root
        .join(".nsc")
        .join("recovery")
        .join(run_scope)
        .join(format!("{label}.json"))
}

pub fn write_record(
    repo_root: &Path,
    run_scope: &str,
    label: &str,
    record: &RecoveryRecord,
) -> Result<PathBuf> {
    let path = recovery_path(repo_root, run_scope, label);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(record)?;
    std::fs::write(&path, format!("{json}\n"))?;
    Ok(path)
}
