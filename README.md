# Night Shift Coordinator

Night Shift Coordinator, abbreviated `NSC`, is a local Rust CLI for supervising
prepared Night Shift slice execution.

It solves one concrete problem: long Codex runs can stop, hang, or require
human follow-up while nobody is watching. NSC runs prepared slices in isolated
worktrees, surfaces what is happening, and stops in predictable places.

## Start Here

Use NSC when:

- you already prepared slice markdown files
- you want one worktree per slice
- you want deterministic batch execution from fetched `origin/main`
- you want recovery breadcrumbs when a run stops

Do not use NSC when:

- you need slice preparation
- you need dependency inference
- you need a general workflow engine
- you want NSC to repair incomplete or local-only input

Core product sentence:

**NSC executes prepared Night Shift work. It does not prepare, repair, or
reinterpret that work.**

## LLM Summary

If you are an LLM reading this README, the core rules are:

- one slice = one markdown file
- `nsc run <slice-folder>` = flat batch mode
- `nsc run <wave-root> --schedule wave-dirs` = ordered child wave-dir mode
- `nsc run <program-root> --schedule wave-roots` = ordered wave-root mode
- flat mode assumes slices in one folder are already parallel-safe
- `wave-dirs` runs one child `batchNN_name` directory at a time
- `wave-roots` runs one child `waveNN_name` directory at a time
- NSC runs from fetched `origin/main` and a resolved base SHA
- prepared slice files must already exist on that execution truth
- `NSC_RESULT:` controls ordered progression
- `NSC_PROGRESS:` and `NSC_EVENT:` are informational only and appear only in
  `nsc debug ...`
- NSC does not copy slice files into worktrees to compensate for weak input

## Supported Use Cases

NSC currently supports:

- run one prepared slice
- run up to 3 independent prepared slices in parallel from one folder
- run ordered wave-dirs one after another with `--schedule wave-dirs`
- run ordered wave roots one after another with `--schedule wave-roots`
- run several parallel-safe slices inside one wave-dir
- run several prepared wave roots that each already use `wave-dirs`
- stop before the next ordered wave-dir if the current one did not land cleanly
- stop before the next ordered wave root if the current one did not land cleanly
- leave recovery info under `.nsc/` for manual follow-up

NSC does not currently support:

- preparing slices
- auto-discovering dependency order
- changing the active batch while a run is already in progress
- restarting an old NSC run automatically
- acting like a full GoalCoder-style orchestrator

## Quick Start

Install locally from this repository:

```bash
cargo install --path . --force
```

Run NSC from the root of the target repository:

```bash
nsc run <slice-folder>
```

Use debug mode when you want live lifecycle and phase lines:

```bash
nsc debug <slice-folder>
```

Use ordered wave-dirs mode when Day Shift already prepared one parent folder of
ordered child batches:

```bash
nsc run <wave-root> --schedule wave-dirs
```

Use ordered wave-roots mode when Day Shift already prepared one parent folder
of ordered wave roots:

```bash
nsc run <program-root> --schedule wave-roots
```

## Commands

Default commands:

```bash
nsc run <slice-folder>
nsc debug <slice-folder>
```

With prompt override:

```bash
nsc run <slice-folder> --prompt-template <path>
nsc debug <slice-folder> --prompt-template <path>
```

With ordered wave-dirs scheduling:

```bash
nsc run <wave-root> --schedule wave-dirs
nsc debug <wave-root> --schedule wave-dirs
```

With ordered wave-roots scheduling:

```bash
nsc run <program-root> --schedule wave-roots
nsc debug <program-root> --schedule wave-roots
```

## Core Contract

These rules define how NSC behaves.

- the current working directory must be the target repo root
- the supplied target path must be inside that repository
- NSC force-fetches `origin/main`, resolves an exact base SHA, and creates
  worktrees from that frozen base
- prepared slice files must already exist on the repository state NSC executes
  from
- once NSC discovers slice files for the current run, treat them as immutable
  for that run
- NSC does not copy slice files into worktrees to compensate for incomplete
  prepared input
- `NSC_RESULT:` is authoritative for ordered progression
- `NSC_PROGRESS:` is debug-only informational output

## Scheduling Modes

NSC has three execution modes.

### Flat Batch Mode

Command:

```bash
nsc run <slice-folder>
```

Contract:

- one batch = one folder
- one slice = one markdown file
- every non-empty `*.md` file in that folder is a slice candidate
- that folder is assumed to contain only slices that are safe to run in
  parallel
- the slice set is frozen at startup
- NSC runs up to 3 slices at a time
- if there are more than 3 slices, the rest queue in lexicographic filename
  order

Minimal example:

```text
slices/
  01_readme_cleanup.md
  02_cli_examples.md
  03_release_notes.md
```

### Ordered Wave-Dirs Mode

Command:

```bash
nsc run <wave-root> --schedule wave-dirs
```

Contract:

- the supplied folder is a wave root
- each immediate child directory is one wave-dir
- wave-dir names must match `batchNN_name`
- numbering must be contiguous inside that root
- `README.md` and hidden entries are allowed in the root
- any other root-level non-directory entry fails fast
- sibling wave-dirs run one at a time in numeric order
- slices inside one wave-dir still use flat-batch behavior
- NSC fetches `origin/main` before each next wave-dir
- NSC unlocks the next wave-dir only after every slice in the current wave-dir
  reports merged success and the reported merge SHA verifies on fetched
  `origin/main`
- wave-root contents are frozen at startup

Minimal example:

```text
repo_materials/night_shift_batches/validation_entrypoint_wave2_php/
  README.md
  batch02_php_vendor_bin_phpunit/
    01_vendor_bin_phpunit.md
  batch03_composer_test_phpunit/
    01_composer_test_phpunit.md
```

### Ordered Wave-Roots Mode

Command:

```bash
nsc run <program-root> --schedule wave-roots
```

Contract:

- the supplied folder is a program root
- each immediate child directory is one wave root
- wave-root names must match `waveNN_name`
- numbering must be contiguous inside that root
- `README.md` and hidden entries are allowed in the root
- any other visible root-level non-directory entry fails fast
- any visible child directory not matching `waveNN_name` fails fast
- sibling wave roots run one at a time in numeric order
- each wave root reuses existing `wave-dirs` behavior unchanged
- NSC fetches `origin/main` before each next wave root
- NSC also still fetches between child wave-dirs inside each wave root
- the next wave root starts only after the current wave root fully satisfies
  the existing merged-success `wave-dirs` contract
- program-root contents are frozen at startup

Minimal example:

```text
repo_materials/night_shift_batches/validation_entrypoint_program/
  README.md
  wave03_ruby/
    batch01_bundle_exec_rspec/
      01_bundle_exec_rspec.md
    batch02_bin_rspec/
      02_bin_rspec.md
  wave04_ruby_minitest_family/
    batch01_bin_rails_test/
      01_bin_rails_test.md
  wave05_elixir/
    batch01_mix_test/
      01_mix_test.md
```

### Mixed Ordered and Parallel Work

Use `--schedule wave-dirs` when some early work must happen in strict order,
but later work can run in parallel.

Structure the batch like this:

```text
repo_materials/night_shift_batches/ruby_wave3/
  batch01_foundation/
    01_foundation.md
  batch02_followup/
    01_followup.md
  batch03_parallel_finish/
    01_docs.md
    02_tests.md
    03_cleanup.md
```

What happens:

1. `batch01_foundation` runs first
2. `batch02_followup` starts only after `batch01_foundation` finishes with
   merged success
3. `batch03_parallel_finish` starts only after `batch02_followup` finishes with
   merged success
4. inside `batch03_parallel_finish`, NSC may run up to 3 slices in parallel

Use separate wave-dirs when:

- later work depends on earlier work landing on `main`
- one slice changes the base that later slices must build on
- you want a hard stop before later work begins

Put several slices in one wave-dir only when:

- they are parallel-safe with each other
- they are merge-safe from the same fetched `origin/main` base
- none of them needs another slice in that same wave-dir to land first

### Separate Prepared Waves

Use `--schedule wave-roots` when you already have several separately prepared
wave roots and want NSC to run them one after another without manual relaunches.

Structure the batch like this:

```text
repo_materials/night_shift_batches/validation_entrypoint_program/
  wave03_ruby/
    batch01_bundle_exec_rspec/
      01_bundle_exec_rspec.md
    batch02_bin_rspec/
      02_bin_rspec.md
  wave04_ruby_minitest_family/
    batch01_bin_rails_test/
      01_bin_rails_test.md
    batch02_bundle_exec_rake_test/
      02_bundle_exec_rake_test.md
  wave05_elixir/
    batch01_mix_test/
      01_mix_test.md
```

What happens:

1. `wave03_ruby` runs first
2. `wave04_ruby_minitest_family` starts only after `wave03_ruby` finishes
   fully merged-success
3. `wave05_elixir` starts only after `wave04_ruby_minitest_family` finishes
   fully merged-success
4. inside each wave root, NSC still uses the existing `wave-dirs` rules

## Two Common Flows

### Parallel-Safe Flat Batch

Use this when one folder contains slices that are already safe to run together.

```bash
git add slices/
git commit -m "Prepare Night Shift slices"
git push origin main
nsc run slices/
```

What happens:

1. NSC freezes the folder membership at startup
2. NSC validates the slice set against the execution base
3. NSC starts up to 3 slices in parallel
4. NSC reports normal completion or attention-needed states

### Ordered Wave-Dirs Batch

Use this when later work must wait until earlier prepared batches land on
`main`.

```bash
git add repo_materials/night_shift_batches/
git commit -m "Prepare ordered Night Shift wave dirs"
git push origin main
nsc run repo_materials/night_shift_batches/validation_entrypoint_wave2_php/ --schedule wave-dirs
```

What happens:

1. NSC freezes sibling wave-dir discovery at startup
2. NSC runs the first child wave-dir only
3. NSC verifies merged-success markers plus fetched `origin/main`
4. NSC starts the next wave-dir only if the previous one landed successfully

## Slice Input

NSC keeps slice input intentionally light.

A slice is just a markdown file. NSC does not require a heavy metadata schema
in v1.

Minimal example:

```md
# 01_auth_cleanup

Implement the auth cleanup described here.

- stay within this slice
- do not widen scope
- stop and report if a missing architecture decision blocks the work
```

Recommended structure:

```md
# 01_auth_cleanup

Goal:
Clean up auth error handling in the current repository.

Scope:
- keep changes inside this slice
- do not widen scope to nearby cleanup

Stop if:
- a missing architecture decision blocks the work
- a missing product-policy decision blocks the work
- the repo workflow is unclear

Expected workflow:
- plan the change
- implement it
- validate it
- open a PR if the repo workflow expects it
```

## Batch Preparation Rules

Good flat batch:

- `01_readme_cleanup.md`
- `02_cli_examples.md`
- `03_release_notes.md`

Bad flat batch:

- `01_schema_change.md`
- `02_consumer_update.md`

If `02_consumer_update.md` depends on `01_schema_change.md` landing first, they
do not belong in the same flat NSC run.

For ordered work, use one wave root plus `--schedule wave-dirs`.

## Prompt Contract

NSC ships with a built-in single-slice prompt in `NSC_PROMPT.md`.

By default, NSC:

- uses that built-in prompt
- starts Codex App Server turns with model `gpt-5.4`
- sets reasoning effort to `medium`
- passes the assigned slice file path as a repo-relative path
- tells the worker to read that exact slice file before acting
- follows a bounded single-slice workflow:
  - repo-context pre-plan checkpoint
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
  - GitHub Codex review polling every 60 seconds for up to 10 minutes
  - one same-head retry if needed
  - one second 10-minute poll window if needed

You can replace the built-in prompt with `--prompt-template <path>`.

### Result Markers

Required machine-readable lines for merged success:

```text
NSC_RESULT: MERGED_SUCCESS
NSC_MERGED_MAIN_SHA: <40-hex-sha>
NSC_PR: <number>
```

Common non-success lines:

```text
NSC_RESULT: MANUAL_FOLLOWUP
NSC_RESULT: BLOCKED
NSC_RESULT: STOPPED
```

The built-in prompt satisfies this contract. A custom prompt used with
`--schedule wave-dirs` must satisfy it too, or NSC will stop instead of
unlocking the next wave-dir.

### Progress Markers

The built-in prompt may emit exact standalone progress lines:

```text
NSC_PROGRESS: PLANNING
NSC_PROGRESS: IMPLEMENTING
NSC_PROGRESS: WAITING_FOR_REVIEW
NSC_PROGRESS: FIXING_REVIEW_FINDINGS
NSC_PROGRESS: WAITING_FOR_MERGE
```

Rules:

- these markers are informational only
- they do not affect scheduling or unlock behavior
- NSC surfaces them only in `nsc debug ...`
- NSC prints phase changes only, not elapsed-time heartbeat lines

### Debug Event Markers

The built-in prompt may also emit exact standalone workflow-event lines:

```text
NSC_EVENT: REVIEW_POLLING_STARTED
NSC_EVENT: REVIEW_RETRY_POSTED
NSC_EVENT: FOLLOWUP_PUSHED
NSC_EVENT: MERGE_STARTED
```

These markers are also informational only.

In debug mode they explain long quiet stretches more clearly than a generic
heartbeat. For example, they tell you when a slice has started review polling,
posted a same-head retry, pushed follow-up review fixes, or begun merge.

## Runtime Output

NSC writes plain terminal output for a human operator.

Use `nsc run ...` for quieter output.

Use `nsc debug ...` when you want lifecycle and phase lines while a slice is
still running.

Typical normal output:

```text
Night Shift Coordinator v0.1.0

Found 5 slices in folder
Starting 3 slices: 01_auth_cleanup, 02_docs_pass, 03_runner_fix

[03_runner_fix] finished normally
[02_docs_pass] needs attention
  reason: codex app-server run timed out after 28800 seconds
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/02_docs_pass
```

Typical debug output:

```text
Night Shift Coordinator v0.1.0

Found 1 slices in folder
Starting 1 slices: 01_vendor_bin_phpunit

[01_vendor_bin_phpunit] starting
[01_vendor_bin_phpunit] running
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/01_vendor_bin_phpunit
[01_vendor_bin_phpunit] phase: planning
[01_vendor_bin_phpunit] phase: implementing
[01_vendor_bin_phpunit] phase: waiting for review
[01_vendor_bin_phpunit] event: review polling started
[01_vendor_bin_phpunit] event: review retry posted
```

Meaning of the main runtime lines:

- `finished normally`:
  the App Server turn completed and NSC captured the final prompt message
- `needs attention`:
  launch, runtime, protocol, or timeout handling did not complete normally
- `Starting wave-dir` / `Wave-dir finished` / `Wave-dir stopped`:
  ordered wave-dirs progress in `--schedule wave-dirs` mode
- `Fatal startup failure`:
  NSC rejected the initial batch before launching anything
- `Run stopped`:
  a later internal failure stopped the current run after startup had already
  succeeded
- `starting`, `running`, `phase: ...`:
  debug-only lifecycle lines
- `event: ...`:
  debug-only workflow milestone lines such as review polling, retry posting,
  follow-up push, and merge start

## Runtime Files

NSC creates repo-local runtime files under `.nsc/`:

```text
.nsc/
  worktrees/
    <run_scope>/
      <slice_label>/
  recovery/
    <run_scope>/
      <slice_label>.json
```

Recovery JSON is intentionally small. It stores:

- `slice_file_path`
- `worktree_path`
- `thread_id`
- `status`
- optional `progress`
- optional `progress_updated_at` for the latest progress or debug-event update

This is for manual recovery only. It is not a restart system.

## Manual Recovery

If a slice needs attention, NSC surfaces:

- the slice label
- the reason
- the worktree path
- the thread id when available

That gives you enough context to inspect the worktree and, when appropriate,
resume the thread manually:

```bash
codex resume <thread_id>
```

## Failure and Stop Conditions

Startup failures:

- unreadable or empty slice file in the initial batch
- invalid slice-folder path
- prepared slice file missing from the repository state NSC executes from

Result:

- NSC fails before launch

Mid-run problems:

- App Server timeout
- App Server protocol failure
- launch failure

Result:

- active slice runs continue when possible
- the affected slice is marked as needing attention when runtime handling fails

Wave-dirs problems:

- malformed child directory name
- numbering gap such as `batch02_*` followed by `batch04_*`
- unexpected root-level file besides `README.md` or hidden entries
- missing `NSC_RESULT` markers in final prompt output
- reported merge SHA not found on fetched `origin/main`

Result:

- NSC stops before the next sibling wave-dir starts
- if the stop reason is missing external Codex review response after the retry
  window, that means review timing was incomplete, not that NSC proved the
  slice itself was wrong

## Requirements

NSC v1 officially targets:

- macOS
- Linux

You also need:

- `git`
- `codex` with App Server support available on your `PATH`
- a target repository that you can run NSC from

## Install

Local install from source:

```bash
cargo install --path . --force
```

That typically installs `nsc` into:

```text
~/.cargo/bin/nsc
```

Verify:

```bash
which nsc
nsc --help
```

Local release build without install:

```bash
cargo build --release
./target/release/nsc --help
```

## Current v1 Limits

NSC v1 is intentionally narrow:

- max parallel slices is hardcoded to `3`
- no dependency graph
- no overlap scheduler
- no TUI
- no automatic retry
- no dynamic flat-batch refill after startup
- no supervisor restart or run resume
- no GitHub or merge orchestration beyond the built-in prompt contract

NSC also does not:

- decide whether your slice plan is good
- resolve dependency order for you
- infer wave structure from arbitrary folder names
- judge whether the implementation was semantically correct
- replace repo-specific workflow rules

## Development

Useful local commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
