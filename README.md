# Night Shift Coordinator

Night Shift Coordinator, abbreviated `NSC`, is a local Rust CLI for supervising
parallel Night Shift slice execution.

It solves one concrete problem: long Night Shift runs can stop or hang while no
human is watching. NSC runs several prepared slices in separate worktrees and
surfaces which runs finished normally and which runs need attention.

## What NSC Is

NSC is a parallel execution supervisor.

It does:

- discover slice markdown files in a batch folder
- run up to 3 slices at a time
- fetch `origin/main` and create one git worktree per running slice from that
  remote base
- create one fresh Codex App Server thread per running slice
- record minimal recovery breadcrumbs under `.nsc/`
- print human-readable runtime updates to the terminal

It does not:

- prepare slices
- decide slice order or dependency rules
- decide whether slices are logically safe to run together
- judge implementation correctness
- manage review or merge policy
- resume or restart a previous NSC run

## How It Works

The v1 model is intentionally small:

- one batch = one folder
- one slice = one markdown file
- every non-empty `*.md` file in that folder is treated as a slice candidate
- the batch is assumed to already contain only slices that are safe to run in
  parallel

If there are more than 3 slice files, NSC queues them in lexicographic filename
order.

When a slot frees up, NSC rescans the folder. That means you can add more slice
files while NSC is already running.

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

## Usage

Run NSC from the root of the target repository:

```bash
nsc run <slice-folder>
```

Run the same batch with extra lifecycle output:

```bash
nsc debug <slice-folder>
```

Optional prompt override:

```bash
nsc run <slice-folder> --prompt-template <path>
nsc debug <slice-folder> --prompt-template <path>
```

Important rules:

- the current working directory must be the target repo root
- the slice folder must be inside that repository
- NSC fetches `origin/main` and creates slice worktrees from that remote base
- once NSC discovers a slice file for the current run, treat that file as
  immutable for that run

## Real-Life Flow

Typical operator flow:

1. prepare a target repo with a folder of slice markdown files
2. make sure the files in that folder are safe to run in parallel
3. run `nsc run <slice-folder>` from the target repo root
4. watch the terminal for:
   - slices that finished normally
   - slices that need attention
   - newly discovered slices after rescans
5. inspect `.nsc/recovery/...` and `.nsc/worktrees/...` when a slice needs
   manual follow-up
6. add more slice files to the folder if more parallel-safe work becomes ready

Example:

```bash
cd ~/code/my-repo
mkdir -p slices
$EDITOR slices/01_readme_cleanup.md
$EDITOR slices/02_cli_examples.md
$EDITOR slices/03_docs_pass.md
nsc run slices/
```

If more work becomes ready later:

```bash
$EDITOR slices/04_followup_cleanup.md
$EDITOR slices/05_release_notes.md
```

NSC will discover those files when a slot frees up and add them to the queue.

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
- split dependent work into separate batches

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

## What NSC Assumes

NSC v1 assumes:

- you run `nsc` from the target repo root
- the slice folder is inside that repo
- `git` and `codex` are available
- the slice batch is already safe to run in parallel
- discovered slice files do not change during the run
- repo-local workflow docs are authoritative for implementation behavior

## Prompt Model

NSC ships with a built-in single-slice prompt in `NSC_PROMPT.md`.

By default, NSC:

- uses that built-in prompt
- passes the assigned slice file path to Codex as a repo-relative path
- tells the worker to read that exact slice file before acting
- fetches `origin/main` and creates the slice worktree from that remote base
- drives a single-slice workflow built around:
  - a repo-context pre-plan checkpoint
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
  - bounded GitHub Codex review polling

You can replace the built-in prompt with `--prompt-template <path>`.

Built-in prompt:

- good when you want NSC to encourage a specific Night Shift execution style
- assumes a single-slice flow with planning, implementation, PR creation,
  bounded review polling, and merge only after a GitHub Codex review comment
  begins with `Codex Review: Didn't find any major issues.`

Custom prompt:

- useful when the target repo has a different workflow
- still runs under the same NSC runtime model
- does not change what NSC supervises or what NSC ignores

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
- `Discovered new slice`:
  NSC found a new markdown file during rescan and added it to the queue
- `Fatal startup failure`:
  NSC rejected the initial batch before launching anything
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

Result:

- NSC fails before launch

Mid-run problems:

- unreadable or empty new slice file discovered later
- App Server timeout
- App Server protocol failure
- launch failure

Result:

- bad new slice files are warned and ignored
- active slice runs continue when possible
- the affected slice is marked as needing attention when runtime handling fails

## Current v1 Limits

NSC v1 is intentionally narrow:

- max parallel slices is hardcoded to `3`
- no dependency graph
- no overlap scheduler
- no TUI
- no automatic retry
- no supervisor restart or run resume
- no GitHub or merge orchestration

NSC also does not:

- decide whether your slice plan is good
- resolve dependency order for you
- judge whether the implementation was semantically correct
- replace repo-specific workflow rules

## Development

Useful local commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
