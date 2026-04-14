pub mod app_server;
pub mod batch;
pub mod cli;
pub mod error;
pub mod output;
pub mod recovery;
pub mod slice;
pub mod supervisor;
pub mod terminal_result;
pub mod wave_dirs;
pub mod wave_roots;
pub mod worktree;

use std::path::{Path, PathBuf};

use app_server::CodexAppServerClient;
use error::{NscError, Result};
use output::{OutputMode, TerminalOutput};

pub const BUILTIN_PROMPT_TEMPLATE: &str = include_str!("../NSC_PROMPT.md");

pub async fn run(args: cli::RunArgs, output_mode: OutputMode) -> Result<()> {
    let repo_root = worktree::ensure_repo_root(&std::env::current_dir()?)?;
    let target_root = resolve_target_root(&repo_root, &args.target_root)?;
    let prompt_template = load_prompt_template(args.prompt_template)?;
    let mut output = TerminalOutput::new(output_mode);
    let client = CodexAppServerClient::default();
    let options = supervisor::SupervisorOptions {
        repo_root,
        target_root,
        prompt_template,
        schedule: args.schedule,
    };
    supervisor::run(options, &client, &mut output).await
}

fn resolve_target_root(repo_root: &Path, target_root: &Path) -> Result<PathBuf> {
    if !target_root.is_dir() {
        return Err(NscError::InvalidDirectory(target_root.to_path_buf()));
    }
    let canonical_repo_root = repo_root.canonicalize()?;
    let canonical = target_root.canonicalize()?;
    if !canonical.starts_with(&canonical_repo_root) {
        return Err(NscError::TargetPathOutsideRepo {
            repo_root: canonical_repo_root,
            target_path: canonical,
        });
    }
    Ok(canonical)
}

fn load_prompt_template(override_path: Option<PathBuf>) -> Result<String> {
    if let Some(path) = override_path {
        return std::fs::read_to_string(&path)
            .map_err(|source| NscError::PromptTemplateRead { path, source });
    }
    Ok(BUILTIN_PROMPT_TEMPLATE.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::resolve_target_root;
    use crate::error::NscError;

    #[test]
    fn resolve_target_root_accepts_in_repo_directory() {
        let repo = TempDir::new().unwrap();
        let slice_dir = repo.path().join("slices");
        fs::create_dir_all(&slice_dir).unwrap();

        let resolved = resolve_target_root(repo.path(), &slice_dir).unwrap();

        assert_eq!(resolved, slice_dir.canonicalize().unwrap());
    }

    #[test]
    fn resolve_target_root_rejects_directory_outside_repo() {
        let repo = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        let err = resolve_target_root(repo.path(), outside.path()).unwrap_err();

        match err {
            NscError::TargetPathOutsideRepo {
                repo_root,
                target_path,
            } => {
                assert_eq!(repo_root, repo.path().canonicalize().unwrap());
                assert_eq!(target_path, outside.path().canonicalize().unwrap());
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
