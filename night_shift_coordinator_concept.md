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
- discovering ordered child wave-dirs when explicit scheduling mode requests it
- discovering ordered child wave roots when explicit scheduling mode requests
  it
- validating prepared execution input early
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

Product boundary sentence:

**NSC executes prepared Night Shift work. It does not prepare, repair, or
reinterpret that work.**

## Batch Model

NSC v1 uses a very small execution model.

- one batch = one folder
- one slice = one markdown file
- every `*.md` file in that folder is treated as a slice
- the batch is assumed to already contain only slices that are intended to run
  in parallel

If true execution order matters, the user should split the work into separate
batches or use one of the explicit ordered scheduling modes.
NSC v1 does not infer dependency graphs or overlap rules.

Flat-batch membership is static for one run.
NSC does not admit newly added slice files after the run starts in v1.

Explicit ordered-root scheduling policy:

- `nsc run <wave-root> --schedule wave-dirs`
- immediate child `batchNN_*` directories are wave boundaries
- one sibling wave-dir runs at a time
- slices inside one wave-dir may still use current flat-batch behavior
- NSC fetches `origin/main` between wave-dirs
- NSC unlocks the next wave-dir only after merged-success marker parsing plus
  remote `origin/main` verification

Explicit ordered program scheduling policy:

- `nsc run <program-root> --schedule wave-roots`
- immediate child `waveNN_*` directories are wave-root boundaries
- one sibling wave root runs at a time
- each wave root reuses the existing `wave-dirs` behavior unchanged
- NSC fetches `origin/main` between wave roots
- NSC also still fetches between wave-dirs inside each wave root
- NSC unlocks the next wave root only after the current wave root fully
  satisfies the existing `wave-dirs` merged-success contract

## CLI Surface

Recommended v1 CLI:

```text
nsc run <slice-folder>
nsc run <slice-folder> --prompt-template <path>
nsc run <wave-root> --schedule wave-dirs
nsc run <program-root> --schedule wave-roots
nsc debug <slice-folder>
nsc debug <slice-folder> --prompt-template <path>
nsc debug <wave-root> --schedule wave-dirs
nsc debug <program-root> --schedule wave-roots
```

Simple rule:

- NSC runs from the target repository root
- the target path is the primary required input
- the target path must resolve inside that repository root in v1
- prompt override is optional
- explicit scheduling policy is optional
- `debug` is the only extra top-level command in v1 and only changes output
  verbosity

## Queueing

NSC v1 runs at most 3 slices in parallel.

If a batch has more than 3 slices:

- NSC starts the first 3 in deterministic filename order
- the rest wait in the queue
- when a slot frees up, NSC starts the next slice

Filename ordering is only queue order.
It is not dependency semantics.

For v1, deterministic filename order means lexicographic filename sorting.

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
- NSC should not compensate for incomplete or local-only prepared input

Startup rule:

- if the initial slice-folder scan finds unreadable or empty slice files, NSC
  should fail before launching anything
- flat-batch membership should be frozen from that startup scan
- for `wave-dirs`, sibling wave-dirs should be frozen at startup and slice
  membership should be frozen when each wave-dir begins
- for `wave-roots`, sibling wave roots should be frozen at startup and each
  wave root should still freeze its own child wave-dir and slice discovery
  under the current `wave-dirs` rules

Execution-truth rule:

- a discovered slice file is launchable only if it also exists on the exact
  repository state NSC is executing from
- missing-on-base slice files are validation failures, not runtime surprises
- NSC should not copy slice files into worktrees to compensate for incomplete
  prepared input

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
- require prepared slice files to already exist in the repository state NSC is
  executing from
- keep the built-in prompt single-slice and repo-agnostic
- the built-in prompt may direct the worker through a bounded workflow such as:
  pre-plan checkpoint, `$plan-author-review-loop`,
  `$implementation-new-branch-pr`, and GitHub Codex review polling every 60
  seconds for up to 10 minutes, then one same-head retry and one second
  10-minute poll window if needed
- the built-in prompt may emit exact standalone progress lines for debug-mode
  visibility:
  - `NSC_PROGRESS: PLANNING`
  - `NSC_PROGRESS: IMPLEMENTING`
  - `NSC_PROGRESS: WAITING_FOR_REVIEW`
  - `NSC_PROGRESS: FIXING_REVIEW_FINDINGS`
  - `NSC_PROGRESS: WAITING_FOR_MERGE`
- the built-in prompt may emit exact standalone workflow-event lines for
  debug-mode visibility:
  - `NSC_EVENT: REVIEW_POLLING_STARTED`
  - `NSC_EVENT: REVIEW_RETRY_POSTED`
  - `NSC_EVENT: FOLLOWUP_PUSHED`
  - `NSC_EVENT: MERGE_STARTED`
- those progress lines are informational only and must not affect scheduling or
  unlock behavior
- the built-in prompt may merge the PR when the repo workflow allows it, but
  only after a GitHub Codex review comment for the current PR head begins with
  `Codex Review: Didn't find any major issues.`
- the built-in prompt should emit exact standalone machine-readable terminal
  lines for NSC result parsing:
  - `NSC_RESULT: MERGED_SUCCESS`
  - `NSC_MERGED_MAIN_SHA: <40-hex-sha>`
  - `NSC_PR: <number>`
  - or one non-success `NSC_RESULT:` line such as `MANUAL_FOLLOWUP`
- do not inline slice contents into the prompt in v1
- do not add extra prompt-generation logic in v1

Debug-output rule:

- `nsc debug ...` should show `starting`, `running`, and coarse
  `phase: ...` lines when the prompt emits `NSC_PROGRESS:` markers
- `nsc debug ...` should also show `event: ...` lines when the prompt emits
  `NSC_EVENT:` markers for review and merge milestones
- NSC should not print periodic elapsed-time heartbeat lines in v1.1
- recovery JSON may store the latest progress or debug-event timestamp for manual
  follow-up

NSC must not use the old autonomous multi-slice Night Shift prompt.

Operator rule:

- once a slice has been discovered for the current run, that slice file should
  be treated as immutable for that run
- NSC should not rely on late local file injection to change the active run
- existing queued or running slice files should not be edited in place

## Worktree and Thread Model

For every running slice, NSC creates:

- one separate git worktree from freshly fetched `origin/main`
- one fresh Codex App Server thread

One slice run maps to one thread.
No thread reuse in v1.

NSC should fetch `origin/main` before launching slices and create worktrees
from that remote base without switching or mutating the user's main checkout.

Base-freezing rule:

- NSC should force-refresh the local `origin/main` tracking ref from remote
  truth before resolving a base
- flat mode should resolve one exact commit SHA for the whole run and create all
  flat-batch worktrees from that SHA
- `wave-dirs` mode should resolve one exact commit SHA per wave-dir and create
  all worktrees in that wave-dir from that SHA
- `wave-roots` mode should invoke `wave-dirs` fresh for each wave root, so each
  wave root keeps the existing per-wave-dir base-refresh behavior inside it
- NSC should validate slice existence against that resolved SHA before launch

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

`run_scope` should be collision-resistant.

Recommended shape:

- local timestamp
- milliseconds
- process id

Example:

- `20260412-220501-127-48129`

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
- fatal startup failure
- run stopped

Output rule:

- `Fatal startup failure` is only for failures before any slice launches
- `Run stopped` is for later internal failures after startup already succeeded

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
- writing usable recovery breadcrumbs for manual recovery
- classifying normal completion versus attention-needed outcomes
- freezing flat-batch membership after startup
- freezing active wave-dir membership once that wave starts

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
