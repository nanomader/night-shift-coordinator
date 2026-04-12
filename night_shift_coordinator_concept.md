# Night Shift Coordinator Concept

## Purpose

Night Shift Coordinator, abbreviated `NSC`, is a free and open source local
Rust CLI for supervising parallel Night Shift slice execution.

The first problem it solves is simple:

- long Night Shift runs sometimes stop unexpectedly
- when that happens in the middle of the night, progress stalls until a human
  notices
- NSC reduces that pain by running multiple prepared slices in separate
  worktrees and surfacing when one run needs attention

Reliability and simplicity outrank features.

## Supported Platforms

NSC v1 targets:

- macOS
- Linux

Windows is out of scope for v1.

## Product Boundary

NSC is a parallel execution supervisor.

It is not:

- a planning engine
- a slice generator
- a merge manager
- a correctness orchestrator
- a replacement for Day Shift preparation

NSC owns:

- discovering slice markdown files in a batch folder
- launching up to 3 slice runs at a time
- creating one worktree per running slice
- creating one fresh Codex App Server thread per running slice
- watching runtime state
- recording minimal recovery breadcrumbs
- surfacing final prompt messages or attention-needed conditions

NSC does not own:

- deciding slice order
- deciding whether slices are logically safe to run together
- deciding architecture
- deciding product policy
- deciding whether implementation quality is sufficient
- deciding review or merge readiness
- merging pull requests or merge requests
- restarting or auto-repairing failed runs in v1
- using a small-model recon layer in v1

## Core Principle

Parallelize execution, not decision ownership.

The user prepares slices.
NSC runs them.
The prompt and the repository workflow own everything beyond runtime
supervision.

## Batch Model

NSC v1 uses a very small execution model.

- one batch = one folder
- one slice = one markdown file
- every `*.md` file in that folder is treated as a slice
- the batch is assumed to already contain only slices that are intended to run
  in parallel

If true execution order matters, the user should split the work into separate
batches.
NSC v1 does not enforce dependency graphs or overlap rules.

## CLI Surface

Recommended v1 CLI:

```text
nsc run <slice-folder>
nsc run <slice-folder> --prompt-template <path>
```

Simple rule:

- NSC runs from the target repository root
- the slice folder is the primary required input
- the slice folder must resolve inside that repository root in v1
- prompt override is optional
- do not add more top-level commands in v1

## Queueing

NSC v1 runs at most 3 slices in parallel.

If a batch has more than 3 slices:

- NSC starts the first 3 in deterministic filename order
- the rest wait in the queue
- when a slot frees up, NSC starts the next slice

Filename ordering is only queue order.
It is not dependency semantics.

For v1, deterministic filename order means lexicographic filename sorting.

## Dynamic Batch Growth

NSC v1 should allow the user to add more slice markdown files while NSC is
already running.

Simple rule:

- NSC rescans the batch folder at simple checkpoints
- newly discovered `*.md` files are added to the queue
- no filesystem watcher is required in v1

Recommended checkpoints:

- once at startup
- whenever a slot frees up

This keeps the system simple while supporting the "add more wood to the fire"
workflow.

## Slice Input

NSC v1 keeps slice input intentionally light.

The current direction is:

- one markdown file per slice
- minimal required structure
- examples and templates should teach good slice preparation

NSC should not require a heavy metadata contract in v1.
Planning discipline belongs to preparation, not to a runtime scheduler.

Recommended minimum validity rule:

- a discoverable slice candidate is any non-empty `*.md` file outside `.nsc/`
- launch admission may still reject a discovered candidate before start
- unreadable or empty slice files should fail the initial startup scan
- malformed files discovered later should warn and be ignored until fixed

Startup rule:

- if the initial slice-folder scan finds unreadable or empty slice files, NSC
  should fail before launching anything

Mid-run rule:

- if a later rescan finds unreadable, empty, or otherwise malformed new slice
  files, NSC should warn and ignore those files until they are fixed
- already running slices should continue

## Prompt Model

NSC should ship with one built-in opinionated single-slice prompt template.

Rules:

- the default prompt source file is `NSC_PROMPT.md`
- that file lives in the main folder of the NSC repo
- the prompt should be compiled into the binary for v1
- CLI should allow overriding the prompt template path

Prompt assembly should stay simple:

- start with the built-in or overridden prompt template
- pass the assigned slice file path to the worker as a repo-relative path
- instruct the worker to read that exact slice file before acting
- do not inline slice contents into the prompt in v1
- do not add extra prompt-generation logic in v1

NSC must not use the old autonomous multi-slice Night Shift prompt.

Operator rule:

- once a slice has been discovered for the current run, that slice file should
  be treated as immutable for that run
- new slice files may be added while NSC is running
- existing queued or running slice files should not be edited in place

## Worktree and Thread Model

For every running slice, NSC creates:

- one separate git worktree
- one fresh Codex App Server thread

One slice run maps to one thread.
No thread reuse in v1.

Recommended repo-local layout:

```text
.nsc/
  worktrees/
    <run_scope>/
      <filename_stem>/
  recovery/
    <run_scope>/
      <filename_stem>.json
```

This keeps the runtime state inspectable without building a larger persistence
system.

`run_scope` should be a simple local timestamp string such as
`20260412-220501`.

## Runtime States

NSC v1 should use coarse, honest runtime states only:

- `queued`
- `starting`
- `running`
- `attention_needed`
- `finished_prompt_owned`

These states describe what NSC can actually observe.
They do not claim semantic success, merge readiness, or correctness.

## What Counts as Finished

NSC should treat a slice run as finished when:

- the Codex App Server turn completes normally
- NSC captures the final prompt-owned terminal message

That final message may report:

- success
- blocker
- stop condition
- manual follow-up needed

NSC should surface that message.
It should not over-interpret it.

If runtime or protocol evidence is missing or abnormal, NSC should use
`attention_needed` instead.

## App Server Boundary

NSC v1 should depend only on Codex App Server runtime facts for runtime-state
classification.

That means:

- thread and turn start signals
- turn completion or failure signals
- terminal prompt-message capture

That does not mean:

- checking repository correctness
- checking tests
- checking review state
- checking merge state

Those concerns stay outside NSC's runtime-state boundary in v1.

Recommended implementation boundary:

- App Server integration should sit behind one narrow adapter
- that adapter should emit coarse lifecycle events plus terminal message text
- the supervisor should not parse raw protocol details throughout the codebase

## Recovery

NSC v1 does not support supervisor restart or NSC-run resume.

It should persist only a tiny recovery breadcrumb per started slice:

- `slice_file_path`
- `worktree_path`
- `thread_id`
- `status`

The recovery file should use one fixed JSON shape in v1 with exactly those
fields.

This is only for manual operator recovery, for example:

```text
codex resume <thread_id>
```

Important boundary:

- NSC does not keep a durable implemented-versus-not-implemented ledger in v1
- in-memory queue state only lives for the current live run
- a new NSC process means a new run, not a resumed run
- existing recovery files may be surfaced as warnings, but must not be adopted
  automatically

## Operator Output

NSC v1 uses plain terminal output only.

The output is for a human operator.
It should:

- print a startup summary once
- print meaningful state changes
- avoid constant redraws
- avoid noisy status spam
- print thread id and worktree path when attention is needed

Slice labels shown to the operator should use the filename stem.

Recommended printed event set:

- startup summary
- slice starting
- slice finished normally
- slice needs attention
- new slice discovered
- fatal startup failure

Example shape:

```text
Night Shift Coordinator v0.1.0

Found 9 slices in folder
Starting 3 slices: 01_auth_cleanup, 02_docs_pass, 03_runner_fix

[03_runner_fix] finished normally
[04_cli_notes] starting
[02_docs_pass] needs attention
  thread_id: abc123
  worktree: .nsc/worktrees/20260412-220501/02_docs_pass

Discovered new slice: 10_followup_cleanup.md
```

## Distribution

NSC v1 distribution should include:

- GitHub Releases with macOS and Linux binaries
- Homebrew support

`cargo install` may be offered as a secondary convenience for Rust users.

## Test Strategy

Tests are one of the main reliability levers for NSC v1.

The goal is not broad coverage for its own sake.
The goal is confidence in the small supervisor loop and its filesystem and
runtime contracts.

### Unit tests should cover

- slice discovery from a folder
- `.nsc/` exclusion from slice discovery
- non-empty markdown validity checks
- deterministic lexicographic filename ordering
- queue admission decisions
- worktree path construction
- recovery file path construction
- recovery file creation and update
- runtime state transitions from mocked App Server events

### Integration-style tests should cover

- startup with more than 3 slices
- queueing the remainder after the first 3 launches
- launching the next slice when a slot frees up
- discovering newly added slice files on rescan
- ignoring malformed newly added slice files while continuing existing runs
- writing usable recovery breadcrumbs for manual recovery
- classifying normal completion versus attention-needed outcomes

### Test boundaries for v1

Do test:

- filesystem behavior
- queue behavior
- recovery-file behavior
- App Server event interpretation behind a controlled adapter

Do not require in v1:

- real GitHub workflow tests
- merge-flow tests
- repo-correctness tests
- end-to-end tests against live Codex production services for every run

### Practical recommendation

- use focused unit tests for pure logic and path handling
- use a small number of integration-style tests for the supervisor loop
- isolate App Server interactions behind a narrow adapter so tests can drive
  deterministic fake event streams

## Non-Goals for v1

NSC v1 should not include:

- dependency scheduling
- overlap scheduling
- merge-readiness judgment
- merge execution
- post-merge validation
- TUI
- restart or auto-resume support
- automatic repair of failed runs
- movement of user-authored slice files
- Windows support
- small-model reconnaissance

## Working Conclusion

NSC v1 should stay small.

It is a Rust CLI that supervises prepared markdown slice batches, runs up to 3
slices in parallel in separate worktrees, uses one fresh Codex App Server
thread per slice, surfaces meaningful runtime outcomes, and leaves planning,
correctness, and landing decisions outside the tool.
