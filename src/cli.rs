use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "nsc")]
#[command(about = "Supervise prepared Night Shift slice execution")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Run with normal operator output")]
    Run(RunArgs),
    #[command(about = "Run with debug lifecycle and workflow-event output")]
    Debug(RunArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScheduleMode {
    Flat,
    WaveDirs,
    WaveRoots,
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    /// Target slice folder, wave root, or program root inside the current repo.
    pub target_root: PathBuf,
    /// Replace the built-in prompt template with a custom file.
    #[arg(long)]
    pub prompt_template: Option<PathBuf>,
    /// Scheduling contract for the supplied target root.
    #[arg(long, value_enum, default_value_t = ScheduleMode::Flat)]
    pub schedule: ScheduleMode,
}
