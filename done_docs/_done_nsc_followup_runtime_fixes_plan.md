# NSC Follow-up Runtime Fixes Plan

## Objective

Fix three nearby runtime gaps in NSC v1 without widening product scope:

- fail fast on unreadable startup slice files
- warn and ignore unreadable mid-run slice files
- detect hung App Server runs and surface `attention_needed`
- keep slice-file access worktree-local by requiring repo-local slice folders and using repo-relative slice paths in worker prompts

## Current Gap / Problem Statement

The current branch has three concrete problems:

1. Startup discovery accepts non-empty markdown files based on `metadata.len() > 0`, which does not prove the file is readable by the NSC process.
2. Mid-run rescans use the same admission rule, so unreadable new slice files can be queued instead of being warned and ignored.
3. A Codex App Server run can hang forever because the supervisor waits indefinitely for completion.
4. Worker prompts receive the original slice-file path instead of a worktree-local repo-relative path, which weakens worktree isolation.

These are runtime correctness problems, not new product-scope decisions.

## Locked Decisions

- Keep NSC v1 small. Do not add restart logic, dependency scheduling, or richer persistence.
- Startup validation must fail before launching anything if a startup slice file is unreadable or empty.
- Mid-run rescans must warn and ignore unreadable or empty new slice files.
- Hung runs should be handled with one hardcoded timeout in v1, not a heartbeat system.
- The hardcoded v1 hung-run timeout is 8 hours in production code.
- Slice folders must resolve inside the repo root for v1.
- Worker prompts should receive the slice path relative to the repo root so the same path works inside each worktree.
- Keep the current recovery JSON shape.

## Scope

### In scope

- shared slice-file admission helper for startup and rescan
- startup failure on unreadable or empty files
- rescan warning-and-ignore behavior for unreadable or empty files
- repo-root containment validation for the slice folder
- repo-relative slice-path handling in prompt construction
- one hardcoded App Server completion timeout that converts hung runs into `attention_needed`
- tests for the above behavior
- small concept-doc updates only if wording is now inaccurate

### Out of scope

- supervisor restart or queue recovery after NSC exits
- richer stall heuristics or progress-heartbeat logic
- filesystem watchers
- dependency ordering
- TUI changes
- GitHub / merge workflow changes

## Implementation Steps With Concrete Files

1. Tighten slice-file admission in [src/batch.rs](/Users/USER/code/night-shift-coordinator/src/batch.rs)
   - Add one helper that validates a launchable slice file by:
     - confirming it is a markdown file candidate
     - opening it for reading
     - rejecting empty content
     - returning a `SliceCandidate` only when readable and non-empty
   - Use that helper in both `discover_startup_candidates` and `discover_new_candidates_on_rescan`.
   - Preserve current startup vs rescan behavior:
     - startup returns an error
     - rescan emits warnings and skips the file

2. Enforce repo-local slice folders in [src/lib.rs](/Users/USER/code/night-shift-coordinator/src/lib.rs) and [src/error.rs](/Users/USER/code/night-shift-coordinator/src/error.rs)
   - Canonicalize the provided slice-folder path after repo-root detection.
   - Reject slice folders that are outside the repo root with a dedicated error.
   - Keep the CLI surface unchanged.
   - Keep this check at the top-level run path before supervisor startup so bad slice-folder paths fail before any worktree or App Server launch.

3. Pass repo-relative slice paths through the supervisor and App Server request path
   - Update [src/supervisor.rs](/Users/USER/code/night-shift-coordinator/src/supervisor.rs) so each run request carries:
     - the original absolute slice path for recovery bookkeeping
     - the repo-relative slice path for prompt construction
   - Update [src/app_server.rs](/Users/USER/code/night-shift-coordinator/src/app_server.rs) so `build_prompt_text` uses the repo-relative path, not the original host path.
   - Keep recovery files storing the original slice-file path string so operator breadcrumbs remain clear.

4. Add a hardcoded hung-run timeout in [src/app_server.rs](/Users/USER/code/night-shift-coordinator/src/app_server.rs)
   - Add one constant for maximum run duration in v1: 8 hours.
   - Wrap the terminal-outcome wait so a run that never reaches terminal completion resolves to `TerminalRunOutcome::AttentionNeeded` with a clear timeout reason.
   - Ensure the child process is still terminated on timeout before returning.
   - Keep timeout handling behind a small helper so unit tests can exercise the logic with a very short duration.
   - Do not add configurable timeout flags in v1.

5. Add or update tests
   - [tests/batch_discovery.rs](/Users/USER/code/night-shift-coordinator/tests/batch_discovery.rs)
     - startup rejects unreadable non-empty markdown files
     - rescan warns and ignores unreadable non-empty markdown files
     - preserve existing empty-file coverage
   - [tests/supervisor_loop.rs](/Users/USER/code/night-shift-coordinator/tests/supervisor_loop.rs)
     - startup failure on an unreadable slice file emits a fatal startup failure and proves no run was launched
   - unit tests in [src/app_server.rs](/Users/USER/code/night-shift-coordinator/src/app_server.rs)
     - timeout helper converts a hung wait into `attention_needed`
     - prompt text uses repo-relative slice path
   - targeted tests in [src/lib.rs](/Users/USER/code/night-shift-coordinator/src/lib.rs) or a dedicated new test file
     - repo-local slice-folder validation accepts in-repo folders
     - repo-local slice-folder validation rejects out-of-repo folders with the dedicated error

6. Update documentation only where implementation now makes a previously implicit rule explicit
   - [night_shift_coordinator_concept.md](/Users/USER/code/night-shift-coordinator/night_shift_coordinator_concept.md)
     - clarify that the slice folder must be inside the repo root in v1
     - clarify that worker prompts receive repo-relative slice paths

## Tests

### Unit tests

- batch admission helper accepts readable non-empty markdown files
- batch admission helper rejects empty files
- batch admission helper rejects unreadable files on macOS/Linux
- repo-relative slice-path derivation succeeds for in-repo paths
- repo-relative slice-path derivation rejects out-of-repo paths
- App Server prompt builder uses repo-relative slice path
- App Server timeout helper converts a hung wait into `attention_needed`

### Integration-style tests

- startup with an unreadable slice file fails before launching anything and no App Server run starts
- rescan with a newly added unreadable slice file emits a warning and does not queue it

### Manual validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run missing-slices`

## Risks + Mitigations

- Risk: Unix permission-based unreadable-file tests can leave temp files undeletable.
  - Mitigation: restore permissions before temp-dir cleanup and gate these tests with `#[cfg(unix)]`.

- Risk: timeout tests become slow or flaky if they rely on real wall-clock waits.
  - Mitigation: keep timeout logic behind a helper that can be exercised with a very short duration in tests.

- Risk: repo-relative path enforcement can break current behavior for users passing slice folders outside the repo.
  - Mitigation: fail with a dedicated clear error message and document the repo-local rule in the concept doc.

- Risk: changing prompt path semantics could break recovery expectations.
  - Mitigation: keep recovery records storing the original slice-file path and only change the prompt-facing path.

## Done Definition

The work is done when all of the following are true:

- startup discovery rejects unreadable or empty slice files before any launch
- mid-run rescans warn and ignore unreadable or empty new slice files
- slice-folder paths outside the repo root fail with a clear error
- worker prompts reference slice files by repo-relative path
- a hung App Server run resolves to `attention_needed` after the hardcoded timeout
- automated tests cover unreadable-file handling, repo-relative path handling, and timeout handling
- local validation passes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
