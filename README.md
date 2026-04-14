# Night Shift Coordinator

Night Shift Coordinator, abbreviated `NSC`, is a local Rust CLI for supervising
Night Shift slice execution.

It solves one concrete problem: long Night Shift runs can stop or hang while no
human is watching. NSC runs prepared slices in separate worktrees and surfaces
which runs finished normally and which runs need attention.

## NSC Cheat Sheet

In simple terms:

- you prepare coding tasks as markdown files
- NSC runs those tasks in isolated worktrees
- NSC watches the runs and tells you what finished and what needs attention

Supported right now:

- run one prepared slice
- run up to 3 independent slices in parallel from one folder
- run ordered wave-dirs one after another with `--schedule wave-dirs`
- run several slices in parallel inside one wave-dir when they are safe to run
  together
- stop before the next ordered wave-dir if the current one did not land cleanly
- leave recovery info under `.nsc/` for manual follow-up

Not supported right now:

- preparing slices for you
- inventing dependency order automatically
- changing the active batch while NSC is already running
- repairing weak input so an invalid run can still continue
- automatically resuming an old NSC run
- acting like a full GoalCoder-style orchestrator

## What NSC Is

NSC is an execution supervisor for prepared Night Shift work.

It executes prepared work. It does not prepare, repair, or reinterpret that
work.

It does:

- discover slice markdown files in a batch folder
- run up to 3 slices at a time when the batch is parallel-safe
- support one explicit ordered-root mode with `--schedule wave-dirs`
- fetch `origin/main` and create one git worktree per running slice from that
  remote base
- create one fresh Codex App Server thread per running slice
- record minimal recovery breadcrumbs under `.nsc/`
- print human-readable runtime updates to the terminal

It does not:

- prepare slices
- decide slice order or dependency rules
- decide whether slices are logically safe to run together
- repair incomplete prepared input so a run can proceed anyway
- judge implementation correctness
- manage review or merge policy
- resume or restart a previous NSC run

## Quick Start

Install locally from this repository:

```bash
cargo install --path . --force
```

Run NSC from the root of the target repository:

```bash
nsc run <slice-folder>
```

Use debug mode when you want live per-slice lifecycle lines:

```bash
nsc debug <slice-folder>
```

Use ordered wave-dirs mode when Day Shift already prepared one parent folder of
ordered child batches:

```bash
nsc run <wave-root> --schedule wave-dirs
```

## Two Common Flows

### Parallel-safe flat batch

Use this when one folder contains slices that are already safe to run together.

1. prepare one folder of slice markdown files
2. commit and push that prepared folder to `origin/main`
3. run `nsc run <slice-folder>` from the target repo root
4. NSC freezes that batch at startup and runs up to 3 slices at a time

Minimal example:

```bash
git add slices/
git commit -m "Prepare Night Shift slices"
git push origin main
nsc run slices/
```

### Ordered wave-dirs batch

Use this when later work must wait until earlier prepared batches land on
`main`.

1. prepare one wave root whose immediate child directories are `batchNN_name`
2. commit and push that prepared wave root to `origin/main`
3. run `nsc run <wave-root> --schedule wave-dirs`
4. NSC runs one child wave-dir at a time and stops on the first non-success

Minimal example:

```bash
git add repo_materials/night_shift_batches/
git commit -m "Prepare ordered Night Shift wave dirs"
git push origin main
nsc run repo_materials/night_shift_batches/validation_entrypoint_wave2_php/ --schedule wave-dirs
```

## Scheduling Modes

NSC has two execution modes.

### 1. Flat Batch Mode

Command:

```bash
nsc run <slice-folder>
```

Contract:

- one batch = one folder
- one slice = one markdown file
- every non-empty `*.md` file in that folder is treated as a slice candidate
- the folder is assumed to contain only slices that are safe to run in parallel
- the slice set is frozen at startup for that run
- NSC validates that startup slice set against the exact repository state it
  will execute from
- NSC runs up to 3 slices at a time
- if there are more than 3 slice files, NSC queues them in lexicographic
  filename order

Example:

```text
slices/
  01_readme_cleanup.md
  02_cli_examples.md
  03_release_notes.md
```

### 2. Ordered Wave-Dirs Mode

Command:

```bash
nsc run <wave-root> --schedule wave-dirs
```

Contract:

- the supplied folder is a wave root
- each immediate child directory is one wave-dir
- wave-dir names must match `batchNN_name`
- numbering must be contiguous inside the root
- sibling wave-dirs run one at a time in numeric order
- slices inside one wave-dir still use current flat-batch behavior
- NSC fetches `origin/main` before each next wave-dir
- NSC unlocks the next wave-dir only after every slice in the current wave-dir
  reports merged success and the reported merge SHA verifies on fetched
  `origin/main`
- wave-root contents are frozen at startup in this mode

Example:

```text
repo_materials/night_shift_batches/validation_entrypoint_wave2_php/
  README.md
  batch02_php_vendor_bin_phpunit/
    01_vendor_bin_phpunit.md
  batch03_composer_test_phpunit/
    01_composer_test_phpunit.md
  batch04_composer_phpunit_phpunit/
    01_composer_phpunit_phpunit.md
```

## Requirements

NSC v1 officially targets:

- macOS
- Linux

You also need:

- `git`
- `codex` with App Server support available on your `PATH`
- a target repository that you can run NSC from

## Install

### Local install from source

From this repository:

```bash
cargo install --path . --force
```

That installs the `nsc` binary into your Cargo bin directory, typically:

```text
~/.cargo/bin/nsc
```

You can verify the install with:

```bash
which nsc
nsc --help
```

### Local release build without install

```bash
cargo build --release
./target/release/nsc --help
```

## CLI Usage

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

Important rules:

- the current working directory must be the target repo root
- the slice folder must be inside that repository
- NSC force-fetches `origin/main`, resolves an exact base SHA, and creates
  slice worktrees from that frozen base
- prepared slice files must already exist on the repository state NSC executes
  from
- once NSC discovers a slice file for the current run, treat that file as
  immutable for that run
- NSC does not copy slice files into worktrees to compensate for incomplete
  prepared input

Wave-dirs mode rules:

- immediate child wave directories must match `batchNN_name`
- `README.md` and hidden entries are allowed in the root
- any other root-level non-directory entry fails fast
- sibling wave-dirs are discovered once at startup in this mode
- NSC resolves one exact base SHA per wave-dir and validates that wave against
  that SHA before launch

## Real-Life Flow

Typical operator flow:

### Flat Batch Flow

1. prepare a target repo with a folder of slice markdown files
2. make sure the files in that folder are safe to run in parallel
3. make sure the prepared slice files already exist in the repository state NSC
   will execute from
4. run `nsc run <slice-folder>` from the target repo root
5. watch the terminal for:
   - slices that finished normally
   - slices that need attention
6. inspect `.nsc/recovery/...` and `.nsc/worktrees/...` when a slice needs
   manual follow-up

Example:

```bash
cd ~/code/my-repo
mkdir -p slices
$EDITOR slices/01_readme_cleanup.md
$EDITOR slices/02_cli_examples.md
$EDITOR slices/03_docs_pass.md
git add slices/
git commit -m "Prepare Night Shift slices"
git push origin main
nsc run slices/
```

### Ordered Wave-Dirs Flow

1. prepare one root directory whose immediate child directories are ordered
   wave-dirs such as `batch02_*`, `batch03_*`, `batch04_*`
2. make sure the prepared wave-root and slice files already exist in the
   repository state NSC will execute from
3. make sure slices inside one child directory are safe to run, review, and
   land from the same fetched `origin/main` base
4. run `nsc run <wave-root> --schedule wave-dirs`
5. NSC runs the first child wave-dir only
6. after that wave-dir finishes, NSC fetches `origin/main` and verifies the
   reported merged SHA for every slice in that wave-dir
7. NSC starts the next child wave-dir only if the previous one landed
   successfully enough
8. if one wave-dir stops with blocker, manual follow-up, missing marker, or bad
   merge verification, NSC stops and does not continue to the next sibling

Minimal example:

```bash
cd ~/code/my-repo
$EDITOR repo_materials/night_shift_batches/validation_entrypoint_wave2_php/README.md
$EDITOR repo_materials/night_shift_batches/validation_entrypoint_wave2_php/batch02_php_vendor_bin_phpunit/01_vendor_bin_phpunit.md
$EDITOR repo_materials/night_shift_batches/validation_entrypoint_wave2_php/batch03_composer_test_phpunit/01_composer_test_phpunit.md
git add repo_materials/night_shift_batches/
git commit -m "Prepare ordered Night Shift wave dirs"
git push origin main
nsc run repo_materials/night_shift_batches/validation_entrypoint_wave2_php/ --schedule wave-dirs
```

## Slice Files

NSC v1 keeps slice input light.

A slice is just a markdown file. NSC does not require a heavy metadata schema in
v1.

Minimal example:

```md
# 01_auth_cleanup

Implement the auth cleanup described here.

- stay within this slice
- do not widen scope
- stop and report if a missing architecture decision blocks the work
```

Recommended practice:

- use one file per slice
- give files stable numeric prefixes when queue order matters
- put only mutually parallel-safe slices in one batch folder
- use `--schedule wave-dirs` when you already have prepared ordered child
  batches

## Slice Template

You do not need a heavy schema in v1, but a little structure helps both humans
and models.

Example:

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

## Good Batch vs Bad Batch

Good batch:

- `01_readme_cleanup.md`
- `02_cli_examples.md`
- `03_release_notes.md`

These slices are mostly independent and can be supervised in parallel.

Bad batch:

- `01_schema_change.md`
- `02_consumer_update.md`

If `02_consumer_update.md` depends on the schema change landing first, they do
not belong in the same NSC v1 run. Split them into separate batches.

NSC does not manage dependencies in v1. If a batch is unsafe, that is a slice
preparation problem, not something NSC will fix at runtime.

For prepared ordered work such as GoalCoder wave queues, use one wave root plus
`--schedule wave-dirs` instead of flattening dependent slices into one
parallel-safe batch.

## What NSC Assumes

NSC v1 assumes:

- you run `nsc` from the target repo root
- the slice folder is inside that repo
- `git` and `codex` are available
- the slice batch is already safe to run in parallel
- prepared slice files already exist in the repository state NSC executes from
- discovered slice files do not change during the run
- repo-local workflow docs are authoritative for implementation behavior
- in `--schedule wave-dirs` mode, sibling child directories encode the required
  execution order

## Prompt Model

NSC ships with a built-in single-slice prompt in `NSC_PROMPT.md`.

By default, NSC:

- uses that built-in prompt
- starts Codex App Server turns with model `gpt-5.4`
- sets reasoning effort to `medium`
- passes the assigned slice file path to Codex as a repo-relative path
- tells the worker to read that exact slice file before acting
- fetches `origin/main` and creates the slice worktree from that remote base
- drives a single-slice workflow built around:
  - a repo-context pre-plan checkpoint
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
  - GitHub Codex review polling every 60 seconds for up to 10 minutes, then one same-head retry and one second 10-minute poll window if needed

You can replace the built-in prompt with `--prompt-template <path>`.

Built-in prompt:

- good when you want NSC to encourage a specific Night Shift execution style
- assumes a single-slice flow with planning, implementation, PR creation,
  review polling every 60 seconds for up to 10 minutes, then one same-head
  retry and one second 10-minute poll window if needed, and merge only after a
  GitHub Codex review comment begins with
  `Codex Review: Didn't find any major issues.`

Custom prompt:

- useful when the target repo has a different workflow
- still runs under the same NSC runtime model
- does not change what NSC supervises or what NSC ignores

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

The built-in prompt satisfies that contract. A custom prompt used with
`--schedule wave-dirs` must satisfy it too, or NSC will stop instead of
unlocking the next wave-dir.

## Runtime Output

NSC writes plain terminal output for a human operator.

Use `nsc run ...` for the quieter default output.

Use `nsc debug ...` when you want extra lifecycle lines while a slice is still
running.

Example shape:

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

Wave-dirs example:

```text
Night Shift Coordinator v0.1.0

Found 3 wave-dirs in root

Starting wave-dir 1/3: batch02_php_vendor_bin_phpunit
Found 1 slices in folder
Starting 1 slices: 01_vendor_bin_phpunit

[01_vendor_bin_phpunit] finished normally
  final_message: ...
Wave-dir 1/3 finished: batch02_php_vendor_bin_phpunit (MERGED_SUCCESS)
Starting wave-dir 2/3: batch03_composer_test_phpunit
```

Debug example:

```text
Night Shift Coordinator v0.1.0

Found 1 slices in folder
Starting 1 slices: 01_vendor_bin_phpunit

[01_vendor_bin_phpunit] starting
[01_vendor_bin_phpunit] running
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/01_vendor_bin_phpunit
[01_vendor_bin_phpunit] still running (60s elapsed)
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
- `starting`, `running`, `still running`:
  debug-only lifecycle lines shown by `nsc debug ...`

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

## When Something Goes Wrong

Startup problems:

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

Execution-truth problems:

- a slice file exists in the operator checkout but not on the frozen execution
  base
- local `origin/main` was stale or rewritten until NSC refreshed it

Result:

- NSC refreshes `origin/main`, resolves the exact base SHA, and validates the
  prepared input against that base before launch
- if the slice set does not exist on that execution truth, NSC fails instead of
  compensating for it

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
