use clap::Parser;
use night_shift_coordinator::cli::{Cli, Command, ScheduleMode};

#[test]
fn parses_run_command() {
    let cli = Cli::parse_from(["nsc", "run", "slices"]);

    match cli.command {
        Command::Run(args) => {
            assert_eq!(args.target_root.as_os_str(), "slices");
            assert_eq!(args.schedule, ScheduleMode::Flat);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_debug_command() {
    let cli = Cli::parse_from(["nsc", "debug", "slices"]);

    match cli.command {
        Command::Debug(args) => {
            assert_eq!(args.target_root.as_os_str(), "slices");
            assert_eq!(args.schedule, ScheduleMode::Flat);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_run_command_with_wave_dirs_schedule() {
    let cli = Cli::parse_from(["nsc", "run", "waves", "--schedule", "wave-dirs"]);

    match cli.command {
        Command::Run(args) => {
            assert_eq!(args.target_root.as_os_str(), "waves");
            assert_eq!(args.schedule, ScheduleMode::WaveDirs);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_debug_command_with_wave_dirs_schedule() {
    let cli = Cli::parse_from(["nsc", "debug", "waves", "--schedule", "wave-dirs"]);

    match cli.command {
        Command::Debug(args) => {
            assert_eq!(args.target_root.as_os_str(), "waves");
            assert_eq!(args.schedule, ScheduleMode::WaveDirs);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_run_command_with_wave_roots_schedule() {
    let cli = Cli::parse_from(["nsc", "run", "program", "--schedule", "wave-roots"]);

    match cli.command {
        Command::Run(args) => {
            assert_eq!(args.target_root.as_os_str(), "program");
            assert_eq!(args.schedule, ScheduleMode::WaveRoots);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_debug_command_with_wave_roots_schedule() {
    let cli = Cli::parse_from(["nsc", "debug", "program", "--schedule", "wave-roots"]);

    match cli.command {
        Command::Debug(args) => {
            assert_eq!(args.target_root.as_os_str(), "program");
            assert_eq!(args.schedule, ScheduleMode::WaveRoots);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
