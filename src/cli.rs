use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "nsc")]
#[command(about = "Night Shift Coordinator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Run(RunArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    pub slice_folder: PathBuf,
    #[arg(long)]
    pub prompt_template: Option<PathBuf>,
}
