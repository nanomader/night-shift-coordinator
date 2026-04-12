use std::path::{Path, PathBuf};
use std::process::Command;

use time::OffsetDateTime;
use time::macros::format_description;

use crate::error::{NscError, Result};

pub const DEFAULT_REMOTE_NAME: &str = "origin";
pub const DEFAULT_BASE_REF: &str = "origin/main";

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

pub fn prepare_default_base_ref(repo_root: &Path) -> Result<String> {
    fetch_origin_main(repo_root)?;
    resolve_base_ref(repo_root, DEFAULT_BASE_REF)
}

pub fn create_worktree(
    repo_root: &Path,
    run_scope: &str,
    label: &str,
    base_ref: &str,
) -> Result<PathBuf> {
    let path = worktree_path(repo_root, run_scope, label);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(&path)
        .arg(base_ref)
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(path)
}

fn fetch_origin_main(repo_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args([
            "fetch",
            DEFAULT_REMOTE_NAME,
            "refs/heads/main:refs/remotes/origin/main",
        ])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

fn resolve_base_ref(repo_root: &Path, base_ref: &str) -> Result<String> {
    let candidate = format!("{base_ref}^{{commit}}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &candidate])
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(NscError::Git(format!(
            "unable to resolve base ref {base_ref}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(base_ref.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;

    use tempfile::TempDir;

    use super::{create_worktree, prepare_default_base_ref};

    #[test]
    fn prepare_default_base_ref_fetches_origin_main_without_moving_local_head() {
        let workspace = init_repo_with_origin();
        let local_head_before = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);
        let latest_origin_head = advance_origin_main(&workspace.source_repo, &workspace.clone_repo);

        let base_ref = prepare_default_base_ref(&workspace.clone_repo).unwrap();
        let local_head_after = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);
        let fetched_origin_head = git_stdout(&workspace.clone_repo, ["rev-parse", "origin/main"]);

        assert_eq!(base_ref, "origin/main");
        assert_eq!(local_head_before, local_head_after);
        assert_ne!(local_head_before, latest_origin_head);
        assert_eq!(fetched_origin_head, latest_origin_head);
    }

    #[test]
    fn create_worktree_uses_requested_base_ref() {
        let workspace = init_repo_with_origin();
        let latest_origin_head = advance_origin_main(&workspace.source_repo, &workspace.clone_repo);
        let base_ref = prepare_default_base_ref(&workspace.clone_repo).unwrap();

        let worktree = create_worktree(
            &workspace.clone_repo,
            "20260412-220501",
            "01_slice",
            &base_ref,
        )
        .unwrap();
        let worktree_head = git_stdout(&worktree, ["rev-parse", "HEAD"]);
        let local_head = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);

        assert_eq!(worktree_head, latest_origin_head);
        assert_ne!(worktree_head, local_head);
    }

    struct RepoWorkspace {
        _temp: TempDir,
        source_repo: PathBuf,
        clone_repo: PathBuf,
    }

    fn init_repo_with_origin() -> RepoWorkspace {
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

        RepoWorkspace {
            _temp: temp,
            source_repo,
            clone_repo,
        }
    }

    fn advance_origin_main(source_repo: &Path, clone_repo: &Path) -> String {
        fs::write(source_repo.join("README.md"), "advanced").unwrap();
        git_ok(source_repo, ["add", "README.md"]);
        git_ok(
            source_repo,
            [
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test User",
                "commit",
                "-m",
                "advance",
            ],
        );
        git_ok(source_repo, ["push", "origin", "main"]);
        let local_head_before_fetch = git_stdout(clone_repo, ["rev-parse", "HEAD"]);
        let remote_head = git_stdout(source_repo, ["rev-parse", "HEAD"]);
        assert_ne!(local_head_before_fetch, remote_head);
        remote_head
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
}
