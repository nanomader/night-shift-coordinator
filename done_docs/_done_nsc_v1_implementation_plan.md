# NSC V1 Implementation Plan

## Objective

Implement the first usable version of Night Shift Coordinator as a Rust CLI
that supervises prepared markdown slice batches, runs up to 3 slices in
parallel in separate git worktrees, launches one fresh Codex App Server thread
per slice, writes minimal recovery breadcrumbs, and prints plain human-oriented
terminal output.

## Current Gap / Problem Statement

The repo currently contains only the concept document.
There is no Rust project scaffold, no CLI entrypoint, no worktree management,
no App Server integration layer, no supervisor loop, no recovery-file support,
and no tests.

That means NSC does not yet solve the concrete problem it was designed for:
supervising multiple Night Shift slice runs so long-running execution does not
silently stall overnight.

## Locked Decisions

- NSC v1 is a free and open source Rust CLI for macOS and Linux.
- NSC v1 runs from the target repository root.
- CLI shape for v1 is:
  - `nsc run <slice-folder>`
  - `nsc run <slice-folder> --prompt-template <path>`
- One batch is one folder.
- One slice is one markdown file.
- Any non-empty `*.md` file outside `.nsc/` is a discoverable slice candidate.
- Launch admission may still reject a discovered candidate before start.
- NSC v1 launches at most 3 slices in parallel.
- Queue order is deterministic lexicographic filename sorting.
- If true slice order matters, the user must split work into separate batches.
- NSC v1 does not implement dependency scheduling or overlap scheduling.
- NSC ships a built-in `NSC_PROMPT.md` prompt template and allows CLI override.
- The prompt should point the worker at the assigned slice file path rather than
  inlining slice contents.
- Each running slice gets one separate worktree and one fresh Codex App Server
  thread.
- Runtime states are:
  - `queued`
  - `starting`
  - `running`
  - `attention_needed`
  - `finished_prompt_owned`
- Runtime classification depends only on App Server lifecycle facts plus
  terminal prompt-message capture.
- Recovery persists one fixed JSON shape with:
  - `slice_file_path`
  - `worktree_path`
  - `thread_id`
  - `status`
- NSC v1 does not support supervisor restart or NSC-run resume.
- A new NSC process means a new run.
- Existing recovery files may be surfaced as warnings but must not be adopted
  automatically.
- Output is plain line-oriented terminal output for a human operator.
- Recommended printed events are:
  - startup summary
  - slice starting
  - slice finished normally
  - slice needs attention
  - new slice discovered
  - fatal startup failure
- Homebrew support is part of the v1 distribution plan, but packaging must not
  distort the core implementation.

## Scope

### In Scope

- Initialize a Rust CLI project in this repo.
- Add the built-in `NSC_PROMPT.md` file.
- Implement `nsc run <slice-folder>` with optional `--prompt-template`.
- Discover batch slice candidates from a folder while excluding `.nsc/`.
- Validate startup slice candidates and fail fast on unreadable or empty files.
- Sort slice candidates lexicographically by filename.
- Queue slices beyond the first 3.
- Create repo-local worktree paths under `.nsc/worktrees/<run_scope>/...`.
- Write recovery JSON files under `.nsc/recovery/<run_scope>/...`.
- Implement a narrow App Server adapter boundary.
- Implement the supervisor loop and runtime state handling.
- Rescan the slice folder at startup and whenever a slot frees up.
- Warn and ignore malformed newly discovered files during rescans.
- Print the human-oriented stdout events defined in the concept doc.
- Add unit tests and integration-style tests for the core contracts.
- Add a minimal `.gitignore` update if needed for Rust build artifacts.

### Out of Scope

- Dependency scheduling.
- Overlap scheduling.
- Merge-readiness or landing logic.
- GitHub PR automation as part of runtime logic.
- TUI.
- Supervisor restart or NSC-run resume.
- Durable “implemented vs not implemented” ledgers.
- Windows support.
- Machine-readable stdout mode.
- Packaging automation beyond what is needed to leave the repo ready for later
  release work.

## Implementation Steps With Concrete Files

### 1. Scaffold the Rust project and core repo files

Create:

- `Cargo.toml`
- `Cargo.lock`
- `src/main.rs`
- `src/lib.rs`
- `NSC_PROMPT.md`

Update if needed:

- `.gitignore`

Notes:

- Use `clap` for CLI parsing.
- Use `tokio` for async runtime and subprocess handling.
- Use `serde` and `serde_json` for recovery-file serialization.
- Use `thiserror` for internal error types.
- Use `tracing` and `tracing-subscriber` for logs if logging is needed beyond
  stdout events.

Local validation contract for this repo should be:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

### 2. Define the core domain modules

Create:

- `src/cli.rs`
- `src/batch.rs`
- `src/slice.rs`
- `src/worktree.rs`
- `src/recovery.rs`
- `src/app_server.rs`
- `src/supervisor.rs`
- `src/output.rs`
- `src/error.rs`

Responsibilities:

- `cli.rs`: parse `nsc run <slice-folder>` and optional prompt override path.
- `batch.rs`: discover slice candidates, ignore `.nsc/`, sort lexicographically,
  and rescan for new files.
- `slice.rs`: define slice identity and runtime-state data structures.
- `worktree.rs`: compute and create `.nsc/worktrees/<run_scope>/<filename_stem>`
  paths.
- `recovery.rs`: write the fixed JSON recovery shape.
- `app_server.rs`: narrow adapter for Codex App Server thread/turn lifecycle
  and terminal message capture.
- `supervisor.rs`: own the queue, admission, state transitions, rescans, and
  slot-free handling.
- `output.rs`: render startup and state-change events to stdout.
- `error.rs`: shared error types where explicit typing helps.

### 3. Implement slice discovery and startup validation

Implement in:

- `src/batch.rs`
- `src/slice.rs`

Behavior:

- treat non-empty `*.md` files outside `.nsc/` as discoverable candidates
- fail startup before launching anything if the initial scan finds unreadable or
  empty files
- retain the distinction between discovered candidates and launchable slices
- sort by lexicographic filename

### 4. Implement `.nsc/` filesystem layout

Implement in:

- `src/worktree.rs`
- `src/recovery.rs`

Behavior:

- create `.nsc/worktrees/<run_scope>/<filename_stem>/`
- create `.nsc/recovery/<run_scope>/<filename_stem>.json`
- use a simple timestamp-based `run_scope`
- keep recovery-file contents limited to:
  - `slice_file_path`
  - `worktree_path`
  - `thread_id`
  - `status`

### 5. Implement the built-in prompt model

Implement in:

- `src/cli.rs`
- `src/app_server.rs`
- `NSC_PROMPT.md`

Behavior:

- compile `NSC_PROMPT.md` into the binary
- allow `--prompt-template <path>` override
- pass the assigned slice file path to the worker prompt
- do not inline slice file contents

### 6. Implement the App Server adapter

Implement in:

- `src/app_server.rs`

Behavior:

- hide raw App Server protocol details behind one narrow interface
- launch the local `codex` binary in App Server mode over stdio
- emit coarse lifecycle events and terminal prompt-message text
- support:
  - thread started
  - turn started
  - turn completed with terminal message
  - abnormal termination / missing terminal message
- keep runtime-state classification based on App Server facts only
- fail fast with a clear operator-facing error if the `codex` binary cannot be
  started or the App Server session cannot be initialized

### 7. Implement the supervisor loop

Implement in:

- `src/supervisor.rs`
- `src/output.rs`

Behavior:

- admit up to 3 slices at once
- queue the rest
- mark runtime states using the locked state set
- start the next queued slice when a slot frees up
- rescan the slice folder whenever a slot frees up
- warn and ignore malformed newly discovered files during rescans
- treat existing queued/running slice files as immutable operator-owned inputs
- write or update recovery files as slice state changes

### 8. Implement stdout output behavior

Implement in:

- `src/output.rs`

Behavior:

- print one startup summary
- print one line per meaningful event
- use filename stem as the human-facing slice label
- print thread id and worktree path on attention-needed events
- avoid redraw loops and noisy periodic output

### 9. Add automated tests

Create:

- `tests/batch_discovery.rs`
- `tests/queueing.rs`
- `tests/recovery.rs`
- `tests/supervisor_loop.rs`
- `tests/app_server_adapter.rs`

Add unit tests inside modules where appropriate for smaller pure helpers.

Notes:

- Use temporary directories for filesystem tests.
- Use temporary git repositories for worktree and supervisor tests so the test
  suite never mutates the real repo checkout.
- Prefer deterministic fake App Server event streams over live Codex services.
- Keep tests aligned with the concept doc contracts rather than speculative
  future features.

### 10. Prepare the repo for implementation follow-through

Create:

- `done_docs/`

Reason:

- the follow-up implementation workflow needs a canonical completed-plan
  archive path
- this repo does not have one yet

The implementation phase should move the finished plan into that directory with
the `_done` prefix.

## Tests

### Unit Tests

- slice discovery ignores `.nsc/`
- only non-empty `*.md` files become discoverable candidates
- lexicographic filename ordering is deterministic
- queue admission logic limits active slices to 3
- worktree path construction matches the locked `.nsc/` layout
- recovery file path construction matches the locked `.nsc/` layout
- recovery JSON serialization uses exactly the locked fields
- runtime state transitions behave correctly for mocked App Server outcomes
- startup validation rejects empty slice files deterministically

### Integration-Style Tests

- startup with 5 slices launches 3 and queues 2
- a freed slot starts the next queued slice
- a new slice file added mid-run is discovered on rescan and queued
- a malformed newly added file is warned about and ignored without disrupting
  current runs
- an empty slice file present at startup fails the run before launch
- a normal App Server completion becomes `finished_prompt_owned`
- missing terminal message or abnormal runtime outcome becomes
  `attention_needed`
- recovery breadcrumbs contain thread id, worktree path, slice file path, and
  status after state changes

### Manual Validation

- run `nsc run <slice-folder>` from the repo root against a small temporary test
  batch
- confirm startup summary format
- confirm state-change output is readable and not noisy
- confirm `.nsc/worktrees/...` and `.nsc/recovery/...` are created as expected
- confirm a manual `codex resume <thread_id>` path is visible after an
  attention-needed case

## Risks + Mitigations

### Risk: App Server integration is underdefined

Mitigation:

- keep one narrow adapter boundary
- define the transport as local `codex` App Server over stdio
- drive it with deterministic fake event tests
- keep supervisor logic independent of raw protocol details

### Risk: The supervisor loop quietly grows into a larger orchestrator

Mitigation:

- keep dependency scheduling, overlap scheduling, merge logic, and restart
  logic out of scope
- make tests assert the v1 boundary behavior

### Risk: File discovery and rescans become flaky

Mitigation:

- use deterministic lexicographic sorting
- fail fast at startup
- warn-and-ignore for mid-run malformed discoveries
- cover rescans in integration-style tests

### Risk: Recovery state grows into a resume system

Mitigation:

- keep one fixed JSON recovery shape
- do not store durable implemented/not-implemented queue state
- treat a new process as a new run

### Risk: Output becomes noisy or hard to follow

Mitigation:

- keep stdout event-driven
- avoid redraws and periodic spam
- test the event set against concrete examples

## Done Definition

This plan is done when all of the following are true:

- the repo contains a working Rust CLI project for NSC
- `nsc run <slice-folder>` works with optional `--prompt-template <path>`
- the built-in `NSC_PROMPT.md` is compiled into the binary and can be
  overridden
- startup slice discovery, queueing, rescans, and recovery-file behavior match
  the concept doc
- runtime states are limited to the locked v1 set
- App Server integration is isolated behind a narrow adapter
- local validation passes with:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
- automated tests cover the locked unit and integration-style cases
- manual validation confirms readable stdout and usable recovery breadcrumbs
- the implementation changes are ready to commit on a new branch and open as a
  PR
