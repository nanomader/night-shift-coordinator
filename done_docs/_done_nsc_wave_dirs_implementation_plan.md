# NSC Wave-Dirs Implementation Plan

## 1. Objective

Add one explicit ordered-root scheduling policy:

```text
nsc run <folder> --schedule wave-dirs
nsc debug <folder> --schedule wave-dirs
```

This mode must let NSC execute a prepared root of ordered child batch
directories one wave-dir at a time while keeping the current default flat-batch
behavior unchanged.

## 2. Current Gap / Problem Statement

Current NSC supports only one batch folder whose markdown slices are already
safe to run in parallel. That is not enough for GoalCoder's prepared Night
Shift trees where later batch directories depend on earlier directories landing
on `origin/main`.

The missing behavior is:

- discover immediate child wave-dirs from one root
- run sibling wave-dirs in locked order
- refresh `origin/main` between wave-dirs
- stop before the next wave-dir unless the current wave-dir landed successfully
  enough

## 3. Locked Decisions

- Default `nsc run <folder>` semantics stay unchanged.
- New scheduling policy is explicit:
  - `--schedule wave-dirs`
- In `wave-dirs` mode:
  - the given folder is a wave root
  - only immediate child directories count as waves
  - child wave-dirs must match `^batch[0-9]{2}_[a-z0-9._-]+$`
  - numeric batch ids must be unique and contiguous within the supplied root
  - `README.md` and hidden entries are allowed at the root; any other
    non-directory root entry fails fast
  - sibling wave-dirs are discovered once at startup and never added mid-run
  - only one sibling wave-dir runs at a time
  - slices inside one wave-dir may still run in parallel up to
    `MAX_PARALLEL_SLICES`
  - slices inside one wave-dir must be safe to run, review, and land from the
    same fetched `origin/main` base
- Unlocking the next wave-dir requires:
  - every slice in the current wave-dir emitted `NSC_RESULT: MERGED_SUCCESS`
  - every slice emitted `NSC_MERGED_MAIN_SHA: <40-hex-sha>`
  - every emitted merged-main SHA is an ancestor of freshly fetched
    `origin/main`
- Built-in prompt must emit exact machine-readable `NSC_RESULT`,
  `NSC_MERGED_MAIN_SHA`, and `NSC_PR` lines.
- Custom prompts remain allowed, but `--schedule wave-dirs` requires the same
  output contract.
- Debug heartbeat remains 60 seconds.

## 4. Scope

### In Scope

- CLI support for `--schedule wave-dirs` on both `run` and `debug`
- wave-root discovery and validation
- serial wave-dir orchestration with current per-wave flat-batch behavior
- exact prompt-result parsing for machine-readable terminal markers
- remote `origin/main` ancestry verification before unlocking the next wave-dir
- wave-dir progress output
- docs and tests for the new scheduling policy

### Out of Scope

- changing default flat-batch scheduling behavior
- DAG inference or metadata-driven dependencies
- dynamic sibling wave-dir discovery during an ordered run
- retries or auto-repair policy
- Windows support
- Rust-side parsing of GitHub review comments for merge-readiness

## 5. Implementation Steps With Concrete Files

### Step 1: Extend the CLI surface

Update [src/cli.rs](src/cli.rs) to:

- add a `ScheduleMode` enum with:
  - `Flat`
  - `WaveDirs`
- add `--schedule <mode>` to `RunArgs`
- default to `Flat`
- keep `RunArgs` shared by `run` and `debug`

Update [tests/cli.rs](tests/cli.rs) to
cover:

- default `run` parsing without `--schedule`
- `run --schedule wave-dirs`
- `debug --schedule wave-dirs`

### Step 2: Add wave-root discovery and validation

Add a new module
[src/wave_dirs.rs](src/wave_dirs.rs)
to keep ordered-root logic separate from the flat slice scanner.

This module should:

- discover immediate child entries of a root directory
- allow `README.md` plus hidden entries and ignore them
- fail fast on any other non-directory entry
- validate wave-dir names against `^batch[0-9]{2}_[a-z0-9._-]+$`
- extract numeric batch ids
- sort by batch id
- reject duplicate ids
- reject gaps inside the discovered sequence
- fail fast when no valid child wave-dirs are found
- return a typed `WaveDir` list with:
  - directory path
  - directory name
  - numeric batch id

Extend [src/error.rs](src/error.rs)
with wave-dir-specific validation errors so startup failures stay explicit and
operator-readable.

Add tests in a new
[tests/wave_dirs.rs](tests/wave_dirs.rs)
for:

- valid GoalCoder-style roots
- malformed wave-dir names
- duplicate ids
- gaps in ids
- unexpected non-directory root files
- allowed `README.md` and hidden entries

### Step 3: Add machine-readable terminal-result parsing

Add a new module
[src/terminal_result.rs](src/terminal_result.rs)
to parse exact standalone terminal lines from the final prompt message.

The parser should support:

- `NSC_RESULT: MERGED_SUCCESS`
- `NSC_RESULT: MANUAL_FOLLOWUP`
- `NSC_RESULT: BLOCKED`
- `NSC_RESULT: STOPPED`
- `NSC_MERGED_MAIN_SHA: <40-hex-sha>`
- `NSC_PR: <number>`

It should:

- ignore non-marker lines
- require exact marker names
- reject malformed SHA or PR values
- return a typed parsed report that wave-dir orchestration can use

Keep [src/app_server.rs](src/app_server.rs)
focused on App Server protocol handling. It should continue returning the final
terminal message string. The new parser should sit above that boundary.

Add parser unit tests in
[tests/terminal_result.rs](tests/terminal_result.rs)
for success, non-success, missing markers, malformed SHA, and malformed PR.

### Step 4: Refactor the supervisor into flat-batch and wave-dir orchestration

Refactor [src/supervisor.rs](src/supervisor.rs)
so flat-batch execution remains reusable.

Recommended shape:

- keep one entrypoint that branches on schedule mode
- extract the current flat-batch loop into an internal helper that can run one
  batch folder from a caller-supplied `base_ref` and return a structured batch
  summary
- add a wave-dir loop that:
  - discovers and validates the root once
  - prints wave-dir progress like `wave-dir 1/7`
  - refreshes `origin/main` once before each wave-dir and reuses that exact
    fetched base for every slice started in that wave-dir
  - fails if the current wave-dir has no valid slice files when its turn begins
  - runs the current flat-batch helper against that child directory
  - parses terminal markers from each finished slice
  - verifies merged-main SHAs against fetched `origin/main`
  - stops immediately on the first non-success or unverifiable result

To avoid worktree/recovery collisions in wave-dir mode, derive a unique internal
label for each slice using the wave-dir name plus the slice stem, for example:

```text
batch02_php_vendor_bin_phpunit__01_vendor_bin_phpunit
```

Use that scoped label for worktree and recovery paths while still printing
human-readable wave-dir progress separately.

Update [src/lib.rs](src/lib.rs) to
pass the selected schedule mode into the supervisor options.

### Step 5: Add git helpers for wave-dir unlock verification

Extend [src/worktree.rs](src/worktree.rs)
with a small helper that verifies whether a merged-main SHA is contained in
fetched `origin/main`, using:

```bash
git merge-base --is-ancestor <merged_main_sha> origin/main
```

Do not switch or mutate the user checkout.

Reuse the existing fetch-based base-ref behavior between wave-dirs by calling
the existing `prepare_default_base_ref` for each new wave-dir.

If remote verification fails, return an explicit wave-dir unlock error rather
than silently treating it as a generic finished batch.

Add unit-style git tests in [src/worktree.rs](src/worktree.rs)
or integration coverage in
[tests/wave_dirs.rs](tests/wave_dirs.rs)
to verify:

- an ancestor SHA passes
- a non-ancestor SHA fails
- the check works against a temp repo with a bare `origin`

### Step 6: Extend terminal output for wave-dir progress

Update [src/output.rs](src/output.rs)
to add explicit wave-dir events and readable rendering, for example:

- `Found 7 wave-dirs in root`
- `Starting wave-dir 1/7: batch02_php_vendor_bin_phpunit`
- `Wave-dir 1/7 finished with MERGED_SUCCESS`
- `Wave-dir 2/7 stopped: MANUAL_FOLLOWUP`

Keep the current distinction:

- normal mode prints important operator events only
- debug mode additionally prints slice start/running/heartbeat lines

Add unit tests covering the new wave-dir lines in normal mode and keeping
debug-only slice lines gated behind `debug`.

### Step 7: Update the built-in prompt contract

Update [NSC_PROMPT.md](NSC_PROMPT.md)
so the final response always includes exact standalone machine-readable markers.

Required behavior:

- on merged success, emit:
  - `NSC_RESULT: MERGED_SUCCESS`
  - `NSC_MERGED_MAIN_SHA: <40-hex-sha>`
  - `NSC_PR: <number>`
- on manual follow-up, blocker, or stop, emit exactly one corresponding
  `NSC_RESULT:` line
- keep the existing merge gate:
  - merge only after a GitHub Codex review comment for the current PR head
    begins with `Codex Review: Didn't find any major issues.`

The prompt should make clear that the machine-readable lines must be standalone
lines at the end of the final message so NSC can parse them deterministically.

### Step 8: Update docs

Update [README.md](README.md) to
document:

- `--schedule wave-dirs`
- child wave-dir naming requirements
- allowed root entries
- serial wave-dir execution with parallelism inside one wave-dir
- merged-success unlock requirement
- custom prompt compatibility requirement for this mode

Update [night_shift_coordinator_concept.md](night_shift_coordinator_concept.md)
to add the new explicit scheduling policy without weakening the default flat
model.

Keep [nsc_wave_dirs_decision_memo.md](nsc_wave_dirs_decision_memo.md)
and update it only if implementation details need tightening.

## 6. Tests

### Unit / Narrow Integration

- CLI parsing:
  - schedule flag default + explicit `wave-dirs`
- wave-root discovery:
  - valid roots
  - malformed names
  - duplicate ids
  - gaps
  - allowed root entries
- terminal-result parsing:
  - exact success markers
  - non-success markers
  - malformed SHA / PR
- git unlock verification:
  - ancestor passes
  - non-ancestor fails

### Supervisor / End-to-End Style

Add deterministic temp-repo tests that cover:

- wave-dirs execute in order and never overlap sibling waves
- multiple slices inside one wave-dir still use the current flat-batch runner
- next wave-dir starts only after all slices in current wave-dir report
  `MERGED_SUCCESS` and their SHAs verify on `origin/main`
- stop on first `MANUAL_FOLLOWUP`
- stop on missing machine-readable markers
- stop on failed remote verification
- default flat mode still behaves as before

Use fake App Server clients plus temp git repos with a bare `origin`. Do not
rely on live Codex or GitHub in tests.

### Manual Validation

- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`
- one local smoke run of `nsc run <folder> --schedule wave-dirs` against a temp
  or real prepared tree if practical

## 7. Risks + Mitigations

- Risk: unlocking the next wave-dir from model prose is brittle.
  - Mitigation: parse only exact standalone `NSC_RESULT`, `NSC_MERGED_MAIN_SHA`,
    and `NSC_PR` lines.

- Risk: wave-dir mode could accidentally weaken default flat-batch behavior.
  - Mitigation: keep schedule mode explicit and preserve the current default
    `Flat` path.

- Risk: same slice stem in different wave-dirs could collide in `.nsc/`.
  - Mitigation: derive scoped internal labels from `wave_dir_name +
    slice_stem`.

- Risk: tests around git remotes can become flaky.
  - Mitigation: use temp repos with local bare origins and deterministic commits
    only.

- Risk: custom prompts may omit required result markers.
  - Mitigation: document that `--schedule wave-dirs` requires prompt
    compatibility and stop on missing markers.

## 8. Done Definition

The change is done when all of these are true:

- `nsc run <folder>` still behaves exactly as the current flat-batch mode
- `nsc run <folder> --schedule wave-dirs` runs immediate child wave-dirs in
  ascending batch order
- sibling wave-dirs never overlap
- current per-wave flat-batch behavior still works inside one wave-dir
- NSC refreshes `origin/main` before each new wave-dir
- NSC refuses to unlock the next wave-dir unless every slice in the current
  wave-dir reports `MERGED_SUCCESS` and its merged-main SHA verifies on fetched
  `origin/main`
- built-in prompt emits the required exact machine-readable markers
- README and concept docs describe the new mode correctly
- local validation passes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
