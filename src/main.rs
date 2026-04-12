use std::process::ExitCode;

use clap::Parser;
use night_shift_coordinator::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        night_shift_coordinator::cli::Command::Run(args) => {
            match night_shift_coordinator::run(args).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("nsc: {err}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
