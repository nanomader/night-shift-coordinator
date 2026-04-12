# Night Shift Coordinator Concept

## Purpose

Night Shift Coordinator, abbreviated `NSC`, is a local Python orchestration
tool for running up to 4 already-prepared Night Shift implementation slices in
parallel, with the limit configurable by CLI flag.

Each running slice gets:

- one assigned slice file
- one git worktree
- one branch
- one Codex worker
- one PR
- then stop

NSC exists to improve throughput when Day Shift has already prepared bounded
implementation slices and locked the needed architecture and product-policy
decisions.

Reliability and simplicity outrank features.

## One Supported Mode

NSC supports exactly one mode:

- load prepared slice markdown files
- validate their metadata
- choose runnable slices
- create one worktree per slice
- launch up to the configured concurrency limit, capped at 4
- monitor each worker
- verify deterministic evidence
- report status until slices finish, block, fail, or become ready to merge

NSC does not support:

- serial multi-slice Night Shift loop behavior
- planning new slices
- deciding architecture
- deciding product policy
- inferring missing order or dependencies
- one worker continuing to a second slice
- one PR covering multiple slices

There is no second execution mode.
The concept should stay narrow around this one job.

## Core Principle

Parallelize implementation, not decision ownership.

The user and prepared slice docs own:

- scope
- order
- dependencies
- architecture decisions
- product-policy decisions
- stop boundaries
- required tests
- completion evidence

NSC owns:

- deterministic loading of prepared slices
- validation of slice metadata
- conservative scheduling
- worktree isolation
- worker launch
- state tracking
- deterministic validation of evidence
- concurrency limiting

The small model may assist the orchestrator, but Python code remains the source
of truth for state transitions.

## Inputs

NSC should read prepared slice markdown files from one configured slice folder.

For v1, each slice should be one standalone markdown file with YAML front
matter.

Reason:

- one slice has one source artifact
- machine-readable metadata and human-readable instructions stay together
- review can inspect one file per slice
- there is no separate manifest to drift from the slice body

NSC should reject the batch if the slice folder contains malformed metadata,
duplicate `slice_id` values, or dependency cycles.

## Required Slice Contract

Every slice file must include machine-checkable YAML front matter.

Example:

```yaml
nsc_schema: 1
slice_id: validation-launcher-task-test
title: Exact root task test launcher
base_ref: origin/main
target_branch: main
depends_on:
  - validation-launcher-just-build
blocks_parallel_with: []
exclusive_surfaces:
  - goalcoder_runner/validation_entrypoints.py
allowed_write_surfaces:
  - goalcoder_runner/
  - tests/
  - repository_contracts/
  - repo_materials/docs/current_reference/
required_read_docs:
  - AGENTS.md
  - README.md
  - repo_materials/docs/current_reference/validation_entrypoint_expansion_20260410.md
required_tests:
  - id: launcher-task-test-unit
    command: uv run --with-requirements requirements/ci-test.txt pytest -q tests/test_validation_entrypoints.py -k "task and test"
    artifact: artifacts/nsc/validation-launcher-task-test/launcher-task-test-unit.log
branch_name: ns/validation-launcher-task-test
expected_pr_title_prefix: "Validation launcher: task test"
```

Minimum required fields for v1:

- `nsc_schema`
- `slice_id`
- `title`
- `base_ref`
- `target_branch`
- `depends_on`
- `blocks_parallel_with`
- `exclusive_surfaces`
- `allowed_write_surfaces`
- `required_read_docs`
- `required_tests`
- `branch_name`
- `expected_pr_title_prefix`

Required rules:

- list fields must be present even when empty
- `branch_name` must be exact, not a prefix
- `required_tests` entries must include stable `id`, exact `command`, and
  expected `artifact`
- paths must be repo-relative
- absolute paths are invalid
- paths containing `..` are invalid
- directory surfaces should end with `/`

The markdown body should then contain:

- exact scope
- locked architecture decisions
- locked product-policy decisions
- non-goals
- stop boundaries
- implementation notes
- review requirements
- durable update requirements
- completion evidence requirements
- blocker conditions

NSC must not infer missing contract details from prose if required metadata is
absent.

## Preparation Guidance

NSC depends on strong Day Shift preparation.
Night Shift is still a new concept for many developers, so the slice contract
must teach bounded execution clearly.

Prepared slices should look more like a locked implementation brief than a
wishlist item.

Minimum preparation standard:

- one exact slice per file
- one exact accepted behavior change
- explicit rejected neighboring shapes
- exact files or file areas the slice may touch
- exact tests to run
- exact durable docs or ledgers that must update
- explicit blocker conditions
- explicit stop rule if a missing decision appears

If a specific order is required, it must be written down.
NSC must not infer order from filenames, timestamps, or broad architecture
guessing.

For v1, required order should be expressed only through `depends_on`.
If two slices must land in a specific order, write that dependency explicitly.

Read-only reference for preparation style:

- GoalCoder shows the right level of Day Shift locking before Night Shift
  execution: exact scope, exact order, exact stop conditions, and exact durable
  evidence requirements.
- Useful local reference examples include
  `/Users/USER/code/goalcoder/repo_materials/done_docs/_donePLAN_node_pnpm_filter_test_20260409.md`
  and
  `/Users/USER/code/goalcoder/repo_materials/docs/current_reference/validation_entrypoint_expansion_20260410.md`.

NSC should learn from that preparation discipline without depending on
GoalCoder-specific workflow files.

## Scheduling Model

NSC should run a conservative scheduler.

Scheduler rules:

1. load all slice files from the configured folder
2. validate metadata
3. reject duplicate `slice_id` values
4. build a dependency graph from `depends_on`
5. reject dependency cycles
6. mark malformed or unknown dependencies as blocking
7. mark valid but unfinished dependencies as waiting
8. choose runnable slices whose dependencies are fully satisfied
9. avoid launching slices with overlapping `exclusive_surfaces`
10. avoid launching slices when either slice names the other in
    `blocks_parallel_with`
11. cap active workers at the configured concurrency limit
12. cap the configured concurrency limit at 4

Important rules:

- concurrency is a maximum, not a target
- if only 2 slices are safe to run, NSC runs 2
- a dependency is satisfied only when the upstream slice is complete
- `ready_to_merge` is not enough to satisfy `depends_on`
- `merged` is not enough if post-merge validation has not finished

NSC should use deterministic tie-breaking for equally runnable slices.
For v1, sorting by `slice_id` is sufficient.

## Worktree Model

Each active slice runs in its own git worktree.

Worktrees provide local isolation.
They do not prevent:

- merge conflicts
- semantic conflicts
- stale assumptions after `main` changes
- duplicate edits on shared files

That is why worktrees must be combined with:

- explicit `depends_on`
- explicit `exclusive_surfaces`
- explicit `blocks_parallel_with`
- serialized landing
- deterministic stale-branch checks

## Worker Contract

Each worker gets one exact assignment:

- one slice id
- one slice markdown file
- one worktree path
- one branch name
- one target branch
- one target SHA at launch time

The worker must:

- stay inside the assigned slice
- stay inside allowed write surfaces
- stop on missing architecture or product-policy decisions
- open or update one PR for the assigned slice
- report deterministic evidence pointers
- stop after the assigned slice

The worker must not:

- select another slice
- widen scope because a nearby change looks convenient
- resolve missing product-policy decisions on its own
- open a second PR for the same slice
- continue after reporting final status

## Small Model Role

The user asked for Python code with help from a small model acting as a simple
orchestrator.

That is viable only if the small model stays narrow.

Safe role for the small model:

- summarize worker output
- extract likely branch names, PR numbers, SHAs, files, and test artifact paths
- classify likely next state for Python to verify
- prepare human-readable status summaries
- highlight likely blockers for human inspection

Unsafe role for the small model:

- deciding whether a slice is actually complete
- deciding whether broadened scope is acceptable
- deciding whether missing metadata is close enough
- deciding merge readiness without deterministic checks
- inferring undeclared dependency or order rules

So the document should treat the small model as a helper to the Python
orchestrator, not as the final authority.

## Deterministic Validation

Python code should verify every state transition.

Minimum deterministic checks:

- slice metadata passes schema validation
- dependency graph is acyclic
- recorded worktree exists
- recorded branch exists
- PR exists and points at the expected branch
- current head SHA matches the recorded worker SHA
- changed files stay inside `allowed_write_surfaces`
- `exclusive_surfaces` do not collide across active slices
- required test artifacts exist for the current head
- required review state is clean for the current head
- target branch has not advanced past the validated base without re-check
- merged commit is reachable from `target_branch`
- post-merge required durable updates exist

The small model may point Python to likely evidence.
It must not replace validation.

## State Model

NSC should track each slice with explicit states.

Recommended v1 states:

- `queued`
- `blocked_metadata`
- `blocked_dependency`
- `waiting_dependency`
- `runnable`
- `starting`
- `running`
- `blocked_day_shift`
- `scope_violation`
- `pr_open`
- `reviewing`
- `stale_base`
- `needs_rebase`
- `ready_to_merge`
- `merged`
- `complete`
- `failed`
- `timed_out`

Important interpretations:

- `blocked_metadata` means the slice contract itself is not usable
- `blocked_dependency` means a declared dependency is invalid or unknown
- `waiting_dependency` means the dependency is valid but unfinished
- `blocked_day_shift` means the worker hit a decision that Day Shift should
  have locked first
- `stale_base` means the target branch moved after the slice was reviewed or
  otherwise validated
- `complete` means post-merge validation passed, not just that code exists

## Persistence and Resume

NSC should persist state locally under an NSC-owned directory in the target
repo, for example `.nsc/runs/<run_id>/`.

Persist at least:

- run id
- slice folder path
- loaded slice file hashes
- last observed target branch SHA
- per-slice state
- worktree path
- branch name
- worker id or process id
- PR number
- recorded head SHA
- transition history

Use:

- `state.json` for current snapshot
- `events.jsonl` for append-only transition history

On restart, NSC must re-check external facts before trusting persisted state.
Persisted state is a cache, not proof.

## Merge and Landing

Landing must remain serialized.

Parallel implementation is acceptable.
Parallel landing is not.

For v1, NSC should not merge PRs automatically.
It should report `ready_to_merge`, wait for a human or existing repository
workflow to land the PR, then observe the merge and run post-merge validation.

This keeps v1 out of:

- branch protection policy
- merge strategy policy
- rollback policy
- conflict auto-resolution

## Stale Work After `main` Changes

NSC should record the target SHA when each worker starts.

If `main` moves while a worker is running:

- the worker may continue its assigned slice
- the PR may remain open
- the slice must not become `ready_to_merge` without fresh validation
- if the branch no longer cleanly relates to the current target SHA, mark
  `needs_rebase`

V1 should not auto-rebase or auto-resolve conflicts.
That work can change semantics and should remain explicit.

## Unhappy Paths and Edge Cases

This area needs more design attention because most reliability failures will
come from edge cases, not from the happy path.

### Slice Preparation Problems

- missing YAML front matter
- unknown required field
- malformed field type
- duplicate `slice_id`
- duplicate `branch_name`
- empty `required_tests`
- missing stop boundaries in the body
- vague scope that still contains multiple implementation targets

V1 stance:

- fail fast on machine-checkable metadata errors
- mark ambiguous but syntactically valid slices for human correction before run
- do not let the small model repair bad slice contracts

### Dependency Problems

- dependency cycle
- dependency points to unknown slice
- dependency points to a slice removed from the batch
- dependency is valid but upstream slice later fails
- two slices are logically ordered but no dependency was declared

V1 stance:

- reject cycles
- block unknown dependencies
- keep valid dependents in `waiting_dependency`
- if required order was not declared, NSC does not infer it
- missing declared order is a Day Shift prep error, not an orchestration job

### Parallelism Problems

- overlapping `exclusive_surfaces`
- one slice blocks another through `blocks_parallel_with`
- both slices forget to declare a real collision
- two slices touch different files but still create semantic conflict
- one slice changes a shared helper that many others rely on

V1 stance:

- enforce declared blockers strictly
- do not attempt semantic conflict detection in v1
- accept that undeclared conflicts can still exist
- keep merge readiness serialized to reduce damage

This is an important limit:
worktrees isolate local filesystems, not architecture.

### Worker Execution Problems

- worker edits files outside allowed surfaces
- worker opens the wrong PR
- worker pushes to the wrong branch
- worker continues after final report
- worker times out
- worker crashes
- worker reports success with missing artifacts
- worker hits an architecture gap and improvises anyway

V1 stance:

- scope violation blocks merge
- branch or PR mismatch blocks state advance
- timeout preserves worktree, branch, PR, and last known state
- missing evidence means no state advance
- missing architecture decision becomes `blocked_day_shift`

### Review and Validation Problems

- review result is ambiguous
- review was clean on an old head SHA, but the branch changed
- tests passed on a stale head
- required artifact path exists but belongs to an old run
- review finding crosses slice boundary

V1 stance:

- ambiguity blocks merge readiness
- review and tests must match the current head SHA
- artifacts need current-run identity, not just existing file paths
- cross-slice findings trigger reassessment instead of silent widening

### Restart and Recovery Problems

- NSC process dies mid-run
- worktree exists but recorded worker id is gone
- PR exists but local state was never updated
- branch was force-pushed outside NSC
- slice file changed after the run started

V1 stance:

- resume by re-checking git, PR, SHA, and artifact reality
- detect slice-file hash drift and require human confirmation or restart
- do not trust branch-local memory over current repo and GitHub state

### Landing Problems

- PR was merge-ready before another PR landed
- merge conflict appears late
- merge succeeds but required durable updates are missing
- merge succeeds but dependent slices were started from old assumptions

V1 stance:

- every merge may stale other open slices
- post-merge validation is required before `complete`
- dependent slices should wait for upstream `complete`, not just PR creation or
  review clean state

### Human Override Problems

- human wants to force a blocked slice to run
- human wants to merge despite scope violation
- human edits the worktree directly during an NSC run

V1 stance:

- any override must be explicit and logged in the run history
- override support should be narrow and probably postponed from earliest v1
- silent manual drift is an auditability risk and should be treated as a state
  hazard

## Smallest Safe v1

The smallest safe version of NSC is:

1. read standalone slice markdown files with YAML front matter
2. validate the contract
3. build the dependency graph
4. schedule runnable slices conservatively
5. enforce a configurable concurrency limit capped at 4
6. create one worktree and one branch per running slice
7. launch one worker per running slice
8. track state and persist it locally
9. verify branch, PR, changed files, tests, and review status deterministically
10. report `ready_to_merge`
11. observe external merge
12. run post-merge validation

Everything else should wait until this narrow loop is reliable.

## Open Questions

The main unresolved design questions now are edge-case questions, not feature
questions:

- What exact schema should define slice body sections beyond front matter?
- How should NSC identify that a test artifact belongs to the current head SHA?
- What exact review signal counts as clean enough for `ready_to_merge`?
- How should manual overrides be recorded without weakening auditability?
- How should NSC detect and surface undeclared cross-slice semantic conflicts?
- What is the best synthetic repo for exercising dependency, conflict, timeout,
  restart, and stale-base behavior before using a real production repo?

## Working Conclusion

NSC should stay narrow.

It is one Python orchestration mode for prepared Night Shift slices.
It runs at most 4 slices in parallel, each in its own worktree, with order only
when Day Shift wrote that order down explicitly.

The small model may help the orchestrator.
Python must still own the truth.

If NSC tries to absorb planning, architecture, or implicit queue reasoning, it
will become harder to trust and harder to debug.
