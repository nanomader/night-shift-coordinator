## Objective

Add an explicit debug execution path so NSC prints extra runtime lifecycle
information only when the operator asks for it.

## Current Gap / Problem Statement

The current local changes make NSC print `starting`, `running`, `thread_id`,
`worktree`, and periodic `still running` heartbeats on every normal
`nsc run ...` invocation.

That solves the observability gap, but it changes the default operator
experience. The user wants these extra lines only when NSC is invoked in a
debug mode, while the standard `run` command should keep its quieter
human-oriented output.

## Locked Decisions

- Keep the existing human-readable stdout model.
- Do not dump raw App Server protocol events or structured debug traces.
- Use an explicit debug command rather than always-on debug output.
- Keep the current extra lifecycle messages:
  - slice starting
  - slice running with `thread_id` and `worktree`
  - periodic `still running` heartbeat
- Keep the heartbeat interval at 60 seconds unless the code already makes a
  safer choice necessary.

## Scope

### In scope

- Add a debug CLI entrypoint that runs the same supervisor flow with verbose
  operator output enabled.
- Keep standard `nsc run ...` output quiet.
- Thread the debug setting through the output layer without changing the core
  scheduling behavior.
- Update tests to cover both normal and debug output behavior.
- Update README usage examples to explain the difference between `run` and the
  debug command.

### Out of scope

- Raw App Server event streaming
- Log files
- Environment-variable-based debug toggles
- TUI work
- Changing queueing, recovery, or App Server runtime semantics

## Implementation Steps

1. Update CLI parsing in [src/cli.rs](src/cli.rs).
   - Add a new `Debug` subcommand with the same arguments as `Run`.
   - Prefer `nsc debug <slice-folder>` over `nsc debug run ...` to keep the CLI
     small unless the current command structure makes that materially harder.

2. Introduce an explicit output mode in [src/output.rs](src/output.rs).
   - Add an `OutputMode` enum with `Normal` and `Debug`.
   - Let `TerminalOutput` store the selected mode.
   - Extract the event-to-lines decision into a small helper so output-mode
     behavior can be unit-tested without capturing process stdout.
   - Keep `StartupSummary`, `SliceFinishedNormally`, `SliceNeedsAttention`,
     `NewSliceDiscovered`, `Warning`, and `FatalStartupFailure` in both modes.
   - Print `SliceStarting`, `SliceRunning`, and `SliceStillRunning` only in
     `Debug` mode.

3. Wire CLI mode selection into the runtime entrypoint in
   [src/main.rs](src/main.rs) and any
   shared command-resolution path in
   [src/lib.rs](src/lib.rs).
   - Reuse the same supervisor logic for both commands.
   - Only the selected terminal output mode should differ.

4. Keep the current supervisor instrumentation in
   [src/supervisor.rs](src/supervisor.rs),
   but make it output-mode-aware through the sink rather than adding
   supervisor-level branching.
   - Do not remove the existing `SliceRunning` / `SliceStillRunning`
     instrumentation unless the output layer no longer needs it.

5. Update tests.
   - Add output-layer unit tests in a dedicated module in
     [src/output.rs](src/output.rs)
     so they verify normal mode suppresses debug-only lines and debug mode
     prints them.
   - Update supervisor-loop recording tests to handle the new CLI/output shape.
   - Add CLI parsing coverage in a new
     [tests/cli.rs](tests/cli.rs) for
     both `nsc run <folder>` and `nsc debug <folder>`.

6. Update documentation in [README.md](README.md).
   - Document normal vs debug command usage.
   - Show one short example of the extra debug lines.

## Tests

### Unit

- CLI parsing:
  - `nsc run slices/`
  - `nsc debug slices/`
- Output formatting:
  - normal mode suppresses `SliceStarting`
  - normal mode suppresses `SliceRunning`
  - normal mode suppresses `SliceStillRunning`
  - debug mode prints all three

### Integration

- Existing supervisor tests continue to pass with the debug-capable output sink.
- Add or update supervisor tests in
  [tests/supervisor_loop.rs](tests/supervisor_loop.rs)
  so they continue to validate launch/queue behavior after the CLI and output
  changes.

### Manual

- `cargo run -- run <slice-folder>` shows startup summary plus terminal outcome
  lines only.
- `cargo run -- debug <slice-folder>` also shows `starting`, `running`, and
  heartbeat lines.

## Risks + Mitigations

- Risk: CLI shape becomes awkward or inconsistent.
  - Mitigation: keep the command surface to `run` and `debug`, each taking the
    same arguments.

- Risk: debug-only events accidentally stay visible in normal mode.
  - Mitigation: centralize the filtering in `TerminalOutput` instead of
    branching in several call sites.

- Risk: tests overfit exact terminal text.
  - Mitigation: keep tests focused on presence/absence of the debug-only event
    categories, not every literal line break.

## Done Definition

- `nsc run <slice-folder>` keeps the quieter default output.
- `nsc debug <slice-folder>` prints the extra lifecycle information.
- The same supervisor behavior is used for both commands.
- README documents the two modes clearly.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test` all pass.
