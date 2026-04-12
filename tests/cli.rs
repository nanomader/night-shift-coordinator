use clap::Parser;
use night_shift_coordinator::cli::{Cli, Command};

#[test]
fn parses_run_command() {
    let cli = Cli::parse_from(["nsc", "run", "slices"]);

    match cli.command {
        Command::Run(args) => assert_eq!(args.slice_folder.as_os_str(), "slices"),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_debug_command() {
    let cli = Cli::parse_from(["nsc", "debug", "slices"]);

    match cli.command {
        Command::Debug(args) => assert_eq!(args.slice_folder.as_os_str(), "slices"),
        other => panic!("unexpected command: {other:?}"),
    }
}
