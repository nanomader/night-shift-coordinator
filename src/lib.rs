pub mod app_server;
pub mod batch;
pub mod cli;
pub mod error;
pub mod output;
pub mod recovery;
pub mod slice;
pub mod supervisor;
pub mod worktree;

use std::path::{Path, PathBuf};

use app_server::CodexAppServerClient;
use error::{NscError, Result};
use output::TerminalOutput;

pub const BUILTIN_PROMPT_TEMPLATE: &str = include_str!("../NSC_PROMPT.md");

pub async fn run(args: cli::RunArgs) -> Result<()> {
    let repo_root = worktree::ensure_repo_root(&std::env::current_dir()?)?;
    let slice_folder = resolve_slice_folder(&repo_root, &args.slice_folder)?;
    let prompt_template = load_prompt_template(args.prompt_template)?;
    let mut output = TerminalOutput::default();
    let client = CodexAppServerClient::default();
    let options = supervisor::SupervisorOptions {
        repo_root,
        slice_folder,
        prompt_template,
    };
    supervisor::run(options, &client, &mut output).await
}

fn resolve_slice_folder(repo_root: &Path, slice_folder: &Path) -> Result<PathBuf> {
    if !slice_folder.is_dir() {
        return Err(NscError::InvalidDirectory(slice_folder.to_path_buf()));
    }
    let canonical_repo_root = repo_root.canonicalize()?;
    let canonical = slice_folder.canonicalize()?;
    if !canonical.starts_with(&canonical_repo_root) {
        return Err(NscError::SliceFolderOutsideRepo {
            repo_root: canonical_repo_root,
            slice_folder: canonical,
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

    use super::resolve_slice_folder;
    use crate::error::NscError;

    #[test]
    fn resolve_slice_folder_accepts_in_repo_directory() {
        let repo = TempDir::new().unwrap();
        let slice_dir = repo.path().join("slices");
        fs::create_dir_all(&slice_dir).unwrap();

        let resolved = resolve_slice_folder(repo.path(), &slice_dir).unwrap();

        assert_eq!(resolved, slice_dir.canonicalize().unwrap());
    }

    #[test]
    fn resolve_slice_folder_rejects_directory_outside_repo() {
        let repo = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        let err = resolve_slice_folder(repo.path(), outside.path()).unwrap_err();

        match err {
            NscError::SliceFolderOutsideRepo {
                repo_root,
                slice_folder,
            } => {
                assert_eq!(repo_root, repo.path().canonicalize().unwrap());
                assert_eq!(slice_folder, outside.path().canonicalize().unwrap());
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
