use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
    Debug(RunArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScheduleMode {
    Flat,
    WaveDirs,
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    pub slice_folder: PathBuf,
    #[arg(long)]
    pub prompt_template: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = ScheduleMode::Flat)]
    pub schedule: ScheduleMode,
}
