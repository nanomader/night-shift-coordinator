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
- create one git worktree per running slice
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

Optional prompt override:

```bash
nsc run <slice-folder> --prompt-template <path>
```

Important rules:

- the current working directory must be the target repo root
- the slice folder must be inside that repository
- once NSC discovers a slice file for the current run, treat that file as
  immutable for that run

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

## Prompt Model

NSC ships with a built-in single-slice prompt in `NSC_PROMPT.md`.

By default, NSC:

- uses that built-in prompt
- passes the assigned slice file path to Codex as a repo-relative path
- tells the worker to read that exact slice file before acting
- drives a single-slice workflow built around:
  - a repo-context pre-plan checkpoint
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
  - bounded GitHub Codex review polling

You can replace the built-in prompt with `--prompt-template <path>`.

## Runtime Output

NSC writes plain terminal output for a human operator.

Example shape:

```text
Night Shift Coordinator v0.1.0

Found 5 slices in folder
Starting 3 slices: 01_auth_cleanup, 02_docs_pass, 03_runner_fix

[03_runner_fix] finished normally
  final_message: completed
[04_followup] starting
[02_docs_pass] needs attention
  reason: codex app-server run timed out after 28800 seconds
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/02_docs_pass
```

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

## Current v1 Limits

NSC v1 is intentionally narrow:

- max parallel slices is hardcoded to `3`
- no dependency graph
- no overlap scheduler
- no TUI
- no automatic retry
- no supervisor restart or run resume
- no GitHub or merge orchestration

## Development

Useful local commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
