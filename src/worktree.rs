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
    let format =
        format_description!("[year][month][day]-[hour][minute][second]-[subsecond digits:3]");
    let pid = std::process::id();
    match now.format(&format) {
        Ok(timestamp) => format!("{timestamp}-{pid}"),
        Err(_) => format!("run-{}-{pid}", now.unix_timestamp_nanos()),
    }
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

pub fn origin_main_contains_commit(repo_root: &Path, commit_sha: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", commit_sha, DEFAULT_BASE_REF])
        .current_dir(repo_root)
        .output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
    }
}

fn fetch_origin_main(repo_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args([
            "fetch",
            DEFAULT_REMOTE_NAME,
            "+refs/heads/main:refs/remotes/origin/main",
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
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn base_ref_contains_repo_relative_path(
    repo_root: &Path,
    base_ref: &str,
    repo_relative_path: &Path,
) -> Result<bool> {
    let object = format!("{base_ref}:{}", repo_relative_path.display());
    let output = Command::new("git")
        .args(["cat-file", "-e", &object])
        .current_dir(repo_root)
        .output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(_) => Ok(false),
        None => Err(NscError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;

    use tempfile::TempDir;

    use super::{
        base_ref_contains_repo_relative_path, create_worktree, origin_main_contains_commit,
        prepare_default_base_ref, run_scope_now,
    };

    #[test]
    fn prepare_default_base_ref_fetches_origin_main_without_moving_local_head() {
        let workspace = init_repo_with_origin();
        let local_head_before = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);
        let latest_origin_head = advance_origin_main(&workspace.source_repo, &workspace.clone_repo);

        let base_ref = prepare_default_base_ref(&workspace.clone_repo).unwrap();
        let local_head_after = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);
        let fetched_origin_head = git_stdout(&workspace.clone_repo, ["rev-parse", "origin/main"]);

        assert_eq!(base_ref, latest_origin_head);
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

    #[test]
    fn origin_main_contains_commit_checks_remote_ancestry() {
        let workspace = init_repo_with_origin();
        let latest_origin_head = advance_origin_main(&workspace.source_repo, &workspace.clone_repo);
        fs::write(workspace.clone_repo.join("LOCAL_ONLY.md"), "local").unwrap();
        git_ok(&workspace.clone_repo, ["add", "LOCAL_ONLY.md"]);
        git_ok(
            &workspace.clone_repo,
            [
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test User",
                "commit",
                "-m",
                "local-only",
            ],
        );
        let local_only_head = git_stdout(&workspace.clone_repo, ["rev-parse", "HEAD"]);
        let _ = prepare_default_base_ref(&workspace.clone_repo).unwrap();

        assert!(origin_main_contains_commit(&workspace.clone_repo, &latest_origin_head).unwrap());
        assert!(!origin_main_contains_commit(&workspace.clone_repo, &local_only_head).unwrap());
    }

    #[test]
    fn prepare_default_base_ref_handles_rewritten_remote_main() {
        let workspace = init_repo_with_origin();
        let initial_head = git_stdout(&workspace.source_repo, ["rev-parse", "HEAD"]);
        let _ = prepare_default_base_ref(&workspace.clone_repo).unwrap();
        let _ = advance_origin_main(&workspace.source_repo, &workspace.clone_repo);

        git_ok(&workspace.source_repo, ["reset", "--hard", &initial_head]);
        fs::write(workspace.source_repo.join("README.md"), "rewritten").unwrap();
        git_ok(&workspace.source_repo, ["add", "README.md"]);
        git_ok(
            &workspace.source_repo,
            [
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test User",
                "commit",
                "-m",
                "rewrite",
            ],
        );
        git_ok(
            &workspace.source_repo,
            ["push", "--force", "origin", "main"],
        );
        let rewritten_head = git_stdout(&workspace.source_repo, ["rev-parse", "HEAD"]);

        let base_ref = prepare_default_base_ref(&workspace.clone_repo).unwrap();
        let fetched_origin_head = git_stdout(&workspace.clone_repo, ["rev-parse", "origin/main"]);

        assert_eq!(base_ref, rewritten_head);
        assert_eq!(fetched_origin_head, rewritten_head);
    }

    #[test]
    fn base_ref_contains_repo_relative_path_checks_path_on_commit() {
        let workspace = init_repo_with_origin();
        let base_ref = prepare_default_base_ref(&workspace.clone_repo).unwrap();

        assert!(
            base_ref_contains_repo_relative_path(
                &workspace.clone_repo,
                &base_ref,
                Path::new("README.md")
            )
            .unwrap()
        );
        assert!(
            !base_ref_contains_repo_relative_path(
                &workspace.clone_repo,
                &base_ref,
                Path::new("slices/01_missing.md")
            )
            .unwrap()
        );
    }

    #[test]
    fn run_scope_now_includes_millisecond_precision_and_pid() {
        let run_scope = run_scope_now();
        let parts = run_scope.split('-').collect::<Vec<_>>();

        assert!(parts.len() >= 4, "unexpected run scope shape: {run_scope}");
        assert!(parts.last().unwrap().parse::<u32>().is_ok());
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
