use std::path::{Path, PathBuf};
use std::process::Command;

use time::OffsetDateTime;
use time::macros::format_description;

use crate::error::{NscError, Result};

pub fn ensure_repo_root(current_dir: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(current_dir)
        .output()?;
    if !output.status.success() {
        return Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let expected = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let actual = current_dir.canonicalize()?;
    let expected = expected.canonicalize()?;
    if actual != expected {
        return Err(NscError::NotRepoRoot { expected, actual });
    }
    Ok(actual)
}

pub fn run_scope_now() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let format = format_description!("[year][month][day]-[hour][minute][second]");
    now.format(&format)
        .unwrap_or_else(|_| "unknown-run-scope".to_string())
}

pub fn worktree_path(repo_root: &Path, run_scope: &str, label: &str) -> PathBuf {
    repo_root
        .join(".nsc")
        .join("worktrees")
        .join(run_scope)
        .join(label)
}

pub fn create_worktree(repo_root: &Path, run_scope: &str, label: &str) -> Result<PathBuf> {
    let path = worktree_path(repo_root, run_scope, label);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(&path)
        .arg("HEAD")
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(path)
}
