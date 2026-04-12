pub mod app_server;
pub mod batch;
pub mod cli;
pub mod error;
pub mod output;
pub mod recovery;
pub mod slice;
pub mod supervisor;
pub mod worktree;

use std::path::PathBuf;

use app_server::CodexAppServerClient;
use error::{NscError, Result};
use output::TerminalOutput;

pub const BUILTIN_PROMPT_TEMPLATE: &str = include_str!("../NSC_PROMPT.md");

pub async fn run(args: cli::RunArgs) -> Result<()> {
    let repo_root = worktree::ensure_repo_root(&std::env::current_dir()?)?;
    let prompt_template = load_prompt_template(args.prompt_template)?;
    let mut output = TerminalOutput::default();
    let client = CodexAppServerClient::default();
    let options = supervisor::SupervisorOptions {
        repo_root,
        slice_folder: args.slice_folder,
        prompt_template,
    };
    supervisor::run(options, &client, &mut output).await
}

fn load_prompt_template(override_path: Option<PathBuf>) -> Result<String> {
    if let Some(path) = override_path {
        return std::fs::read_to_string(&path)
            .map_err(|source| NscError::PromptTemplateRead { path, source });
    }
    Ok(BUILTIN_PROMPT_TEMPLATE.to_string())
}
