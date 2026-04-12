# NSC Execution-Truth Hardening Plan

## Objective

Bring the implementation into line with the current NSC product boundary:

- NSC executes prepared Night Shift work from frozen repo truth
- flat batches are static for one run
- `wave-dirs` membership is frozen and validated per wave
- missing-on-base slice files are startup validation failures
- internal supervisor failures after startup stop the run honestly

## Current Gap / Problem Statement

Current code still behaves like a looser supervisor than the docs now define.

Main gaps:

1. slice files are discovered from the operator checkout but slices run in
   worktrees created from fetched `origin/main`, so a discovered slice can be
   absent from the execution base
2. flat mode still rescans and admits new slice files after startup
3. `wave-dirs` uses the same rescanning batch runner, so active wave-dir
   membership can change after the wave starts
4. `prepare_default_base_ref()` verifies `origin/main^{commit}` but returns the
   symbolic ref name instead of the resolved SHA
5. `git fetch origin refs/heads/main:refs/remotes/origin/main` is not forced,
   so rewritten remote history can abort startup
6. later internal failures in flat mode are still surfaced as fatal startup
   failures instead of a run-level stop
7. `run_scope_now()` is only second-granular and falls back to a shared static
   string

## Locked Decisions

- Do not copy slice files into worktrees.
- Do not inline slice contents into the prompt.
- Prepared slice files must already exist on the repository state NSC executes
  from.
- Flat batch membership is frozen at startup.
- `wave-dirs` root membership is frozen at startup, and slice membership for one
  wave-dir is frozen when that wave begins.
- NSC must force-refresh `origin/main` before resolving the execution base.
- Flat mode uses one exact base SHA per run.
- `wave-dirs` uses one exact base SHA per wave-dir.
- Missing-on-base slice files are validation failures.
- Internal supervisor failures after startup stop the run and should be reported
  as `Run stopped`.
- `attention_needed` remains for slice/App Server/runtime failures, not
  supervisor invariants.
- `run_scope` must be collision-resistant.

## Scope

In scope:

- `src/worktree.rs` base refresh, SHA resolution, execution-truth helpers, and
  run-scope hardening
- `src/batch.rs` startup-only flat batch discovery and removal of flat-mode
  rescan admission
- `src/supervisor.rs` static batch execution, wave-dir freezing, execution-base
  validation, and run-level stop handling
- `src/output.rs` output event changes needed for honest run-level failures
- `src/error.rs` new explicit validation and run-stop error variants if needed
- tests covering frozen base, rewritten history, missing-on-base validation,
  static membership, and run-level stop behavior

Out of scope:

- changing CLI surface
- changing prompt marker semantics
- adding restart/resume support
- reintroducing dynamic flat-batch refill
- slice-copy workarounds
- broad refactors outside the scheduling/runtime path

## Implementation Steps With Concrete Files

### 1. Freeze the execution base to a resolved commit SHA in [src/worktree.rs](src/worktree.rs)

- Change `fetch_origin_main()` to force-update `refs/remotes/origin/main`
  against remote truth.
- Change `resolve_base_ref()` to return the resolved commit SHA from
  `git rev-parse --verify origin/main^{commit}` instead of returning the
  symbolic string `origin/main`.
- Keep `create_worktree()` taking a base reference string, but pass it resolved
  commit SHAs from the supervisor.
- Add one helper that verifies whether a repo-relative slice path exists on a
  given resolved base SHA using git object queries instead of worktree probing.
- Harden `run_scope_now()` to produce a collision-resistant value using local
  timestamp, millisecond precision, and PID, with no constant shared fallback.

### 2. Make flat-batch membership static in [src/batch.rs](src/batch.rs) and [src/supervisor.rs](src/supervisor.rs)

- Keep startup discovery in `discover_startup_candidates()`.
- Remove flat-mode admission of new slice files after startup from
  `run_flat_batch()`.
- Stop using `discover_new_candidates_on_rescan()` in flat mode.
- Remove or retire `discover_new_candidates_on_rescan()` and the associated
  new-slice output path if they are no longer used anywhere after this change.
- Validate the entire startup slice set against the resolved base SHA before any
  launch:
  - readable and non-empty
  - inside repo
  - repo-relative path exists on the resolved base SHA
- Return a clear validation error when a discovered slice is missing from the
  execution base.

### 3. Freeze active wave-dir membership in [src/supervisor.rs](src/supervisor.rs)

- Keep sibling wave-dir discovery once at startup via `discover_wave_dirs()`.
- For each wave-dir:
  - force-fetch `origin/main`
  - resolve one exact base SHA
  - discover that wave-dir’s slice files once
  - validate those slice files against that wave SHA before launching any slice
- Ensure the active wave-dir does not rescan for newly added files while it is
  running.
- Keep unlock verification after the wave completes, using refreshed
  `origin/main` plus merged-main SHA ancestry checks.

### 4. Split startup-fatal from later run-stop output in [src/output.rs](src/output.rs) and [src/supervisor.rs](src/supervisor.rs)

- Add a `RunStopped` output event.
- Change the top-level `run()` path so it does not wrap all flat-batch errors in
  `FatalStartupFailure`.
- Use `FatalStartupFailure` only for failures before any slice launches.
- In flat mode, if a later internal supervisor failure occurs after startup
  succeeded, emit `RunStopped` and stop the run.
- Apply the same distinction to `wave-dirs` so wave stop messages remain
  specific while underlying run-level failures are not mislabeled as startup
  failures.
- Remove the `NewSliceDiscovered` output path because dynamic refill is no
  longer part of the product contract.

### 5. Tighten error semantics in [src/error.rs](src/error.rs)

- Add explicit error variants if the current ones are not precise enough for:
  - slice missing from execution base
  - later internal run stop after startup
- Keep user-facing messages concrete and aligned with the README wording.

### 6. Update and expand tests under [tests](tests)

- Update tests that currently assume flat-mode rescans and `NewSliceDiscovered`
  output.
- Add deterministic integration-style git tests using temporary repos for:
  - force-fetch handling rewritten `origin/main`
  - resolved base SHA staying fixed for one flat run even if `origin/main` moves
    later
  - local-only slice file rejected before flat-mode launch because it is absent
    from the resolved base SHA
  - wave-dir local-only slice rejected before that wave launches
  - wave-dir membership frozen once the wave starts
  - later internal failure reported as `Run stopped`, not `Fatal startup
    failure`
  - collision-resistant `run_scope` output shape or non-collision behavior
- Keep smaller unit tests for:
  - output rendering of `RunStopped`
  - worktree helpers that return resolved SHAs and ancestry checks

## Tests

Unit:

- [src/worktree.rs](src/worktree.rs) helper tests for forced fetch, resolved SHA
  return values, ancestry helpers, and run-scope shape
- [src/output.rs](src/output.rs) rendering tests for `RunStopped` and removal of
  new-slice output

Integration:

- temp git repo tests in [tests/supervisor_loop.rs](tests/supervisor_loop.rs)
  and/or [tests/wave_dir_scheduler.rs](tests/wave_dir_scheduler.rs) for:
  - local-only slice rejection
  - rewritten remote `main`
  - frozen SHA behavior
  - frozen wave-dir membership
  - run-stopped output on late internal failure

Manual:

- `cargo run -- run --help`
- `cargo run -- debug --help`
- one local smoke run of `nsc run <slice-folder>` in a temp repo with prepared
  committed slices
- one local smoke run of `nsc run <wave-root> --schedule wave-dirs` in a temp
  repo with two valid wave-dirs

Validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

## Risks + Mitigations

### Risk: Git-object validation against the base SHA is brittle if implemented ad hoc

Mitigation:

- use one small helper in `src/worktree.rs`
- test it with real temp repos, not only mocked paths

### Risk: Removing rescans breaks existing tests and some current expectations

Mitigation:

- update tests and README-aligned expectations in the same patch
- keep the change explicit in output and error messages

### Risk: Treating later internal failures as run-level stop may reduce throughput

Mitigation:

- keep slice/App Server failures on the existing `attention_needed` path
- only stop the run for supervisor invariant failures such as worktree creation,
  recovery persistence, or execution-base validation breakdowns

### Risk: Wave-dir and flat-mode code paths drift again

Mitigation:

- keep one shared batch execution core
- keep wave-dirs as a thin policy wrapper around base refresh, wave discovery,
  and unlock checks

## Done Definition

The change is done when all of these are true:

1. flat mode no longer admits new slice files after startup
2. `wave-dirs` no longer admits new slice files after one wave begins
3. flat mode validates discovered slices against one resolved base SHA before
   launching anything
4. `wave-dirs` validates each wave against one resolved base SHA before that
   wave launches
5. rewritten remote `main` no longer breaks startup fetch due to non-fast-
   forward rejection
6. worktrees are created from resolved commit SHAs, not symbolic `origin/main`
7. later internal failures emit `Run stopped` instead of `Fatal startup
   failure`
8. `run_scope` no longer risks obvious same-second collisions
9. `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   and `cargo test` all pass
10. the plan file is archived under `done_docs/` after implementation and
    [TASKS_HISTORY.md](TASKS_HISTORY.md) is updated with one sentence referencing
    that archived plan
