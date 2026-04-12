# NSC Decision Locking Steps

This file tracks the high-level discussion order for Night Shift Coordinator
(`NSC`) so we can lock decisions one step at a time before implementation.

Rule:

- discuss one step at a time
- record agreed decisions for that step
- record open questions that block the step
- move to the next step only after the current step is stable enough

## Step Order

### 1. Product Boundary

Define exactly what NSC owns and what it does not own.

Output:

- one explicit boundary between NSC, Day Shift, and the user
- one invariant list of "NSC does" and "NSC does not do"

Status: mostly locked

### 2. Slice Contract

Define the exact required slice metadata and required markdown body sections.

Output:

- one v1 slice schema
- one canonical example slice

Status: mostly locked

### 3. Execution Model

Define scheduling, concurrency, worktree isolation, worker assignment, and
parallelism rules.

Output:

- one scheduling and worker-execution contract

Status: in discussion

### 4. State Machine

Define slice states, meanings, and allowed transitions.

Output:

- one state and transition table

Status: in discussion

### 5. Deterministic Evidence

Define what Python must verify for each state transition.

Output:

- one evidence matrix covering git, filesystem, worker, review, and merge facts

Status: in discussion

### 6. Persistence and Recovery

Define persisted state, restart behavior, stale-state handling, and recovery
rules.

Output:

- one persistence and resume contract

Status: pending

### 7. Merge-Readiness and Landing Policy

Define what `ready_to_merge` means, what "executed to the end" means, and what
landing behavior NSC will or will not automate.

Output:

- one merge-readiness and landing contract

Status: in discussion

### 8. Unhappy Paths and Operator Actions

Define how NSC reacts to metadata failures, invalid dependencies, worker
failures, stale branches, ambiguous review, and manual overrides.

Output:

- one edge-case and operator-policy section

Status: mostly locked

### 9. Smallest Safe v1

Cut the design down to the smallest version that is still trustworthy.

Output:

- one explicit v1 scope
- one explicit out-of-scope list

Status: pending

### 10. Implementation Shape

Only after steps 1 through 9 are locked, map the design into Python modules,
CLI surface, persistence layout, and tests.

Output:

- one implementation plan

Status: in discussion

## Step 1 Draft: Product Boundary

This section is the current discussion draft for step 1.
It should become a locked decision record before we move to step 2.

### Candidate decisions from current discussion

1. NSC is a Python orchestration tool.
2. NSC runs Night Shift slices only after they have already been prepared.
3. NSC may run up to 4 slices in parallel, with the active limit configurable
   by CLI switch.
4. Each active slice gets exactly one separate git worktree.
5. Each active slice gets exactly one Codex App Server thread.
6. Branch, pull request, merge request, or pure-commit workflow details are
   prompt-owned or repo-owned, not NSC-owned.
7. NSC uses a small model such as GPT-5.4-mini for reconnaissance and status
   extraction only.
8. Python code deterministically validates the small model's findings before
   changing slice state.
9. NSC should supervise execution continuity so a Night Shift run does not
   silently stop in the middle of the night.

### Revised interpretation of the main goal

The earlier phrase "executed to the end" was too broad for NSC's intended
scope.

Better interpretation:

- the assigned Codex App Server thread was launched for the assigned slice
- NSC can observe whether the thread is still running, stalled, or terminated
- NSC can detect obvious early stop conditions
- NSC can distinguish "still running", "stopped unexpectedly", and "reached a
  terminal prompt-owned outcome" well enough to avoid silent overnight failure
- NSC may use a small model for lightweight reconnaissance of thread output or
  status, but Python should confirm any state changes it records
- when a run stops unexpectedly, NSC v1 should report the thread id and
  worktree path so the operator can resume manually, for example with
  `codex resume <thread_id>`

NSC is not the authority on:

- whether the slice implementation is semantically correct
- whether tests chosen by the prompt are sufficient
- whether review policy is correct
- whether the prompt made the right merge decision
- whether durable project-specific updates are complete

Those remain prompt-owned or repository-owned concerns, not NSC-owned concerns.

### Draft boundary statement

NSC owns:

- loading prepared slices
- validating slice metadata
- launching one Codex App Server thread per active slice
- creating and tracking one worktree per active slice
- enforcing the active concurrency limit
- supervising thread liveness and run continuity
- recording observable run state
- using a small model for reconnaissance only
- deterministically validating observable state transitions in Python
- detecting obvious unexpected stop conditions
- reporting enough recovery context for manual resume

NSC does not own:

- inventing new slices
- deciding architecture
- deciding product policy
- inferring undeclared order
- letting one worker continue to another slice
- requiring a specific branch, PR, MR, or pure-commit workflow
- deciding whether implementation quality is good enough
- deciding whether tests are sufficient
- deciding whether merge or durable-update policy was satisfied
- automatically recovering or repairing stopped runs in v1
- accepting model judgment without deterministic validation

### Open questions for step 1

These need to be resolved before step 1 is locked:

1. What exact observable outcomes should NSC classify for one slice run:
   `running`, `stalled`, `stopped_unexpectedly`, `prompt_reported_done`,
   something else?
2. What exact evidence can NSC use to detect that a thread stopped
   unexpectedly?
3. Should the small model ever read worker logs directly, or only structured
   worker output and repository state?

## Decision Log

Add agreed decisions here as we lock them.

### Step 1

Locked decisions:

- NSC is a parallel execution supervisor for already-prepared Night Shift
  slices.
- NSC v1 runs up to 4 slices in parallel, with the active limit configurable by
  CLI switch.
- Each active slice gets exactly one separate git worktree.
- Each active slice gets exactly one Codex App Server thread.
- Branch, pull request, merge request, or pure-commit workflow details are
  prompt-owned or repo-owned, not NSC-owned invariants.
- NSC uses a small model such as GPT-5.4-mini for reconnaissance and status
  extraction only.
- Python code deterministically validates observable state transitions before
  NSC records them.
- NSC is not the authority on implementation correctness, test sufficiency,
  review policy, merge policy, or durable project-specific updates.
- NSC v1 should detect and report obvious unexpected stop conditions when that
  is practical.
- NSC v1 should provide manual recovery context, including the thread id and
  worktree path, so the operator can resume the run manually.
- NSC v1 should not auto-restart, auto-recover, or auto-repair stopped runs.

## Step 2 Draft: Slice Contract

This section defines what a prepared slice file must contain before NSC may
launch it.

NSC v1 reminder:

- the first use case is fixing the pain of Codex CLI stopping long Night Shift
  execution
- NSC v1 should stay simple
- planning discipline should be taught through examples and guidance, not
  pushed into a heavy scheduler contract

Meaning of "slice contract":

- the machine-readable metadata NSC needs in order to schedule and launch the
  slice safely
- the human-readable body the worker needs in order to execute the slice as the
  user intended
- the boundary between "required by NSC" and "passed through to the prompt"

GoalCoder inspiration:

- GoalCoder Night Shift preparation is a useful model because Day Shift does
  not hand Night Shift a vague queue item. It hands over a bounded contract with
  objective, locked decisions, scope, out-of-scope rules, tests, and stop
  boundaries.
- Useful read-only examples include:
  - `/Users/USER/code/goalcoder/repo_materials/done_docs/_donePLAN_python_unittest_no_arg_20260329.md`
  - `/Users/USER/code/goalcoder/repo_materials/done_docs/_done_validation_launcher_make_test_plan_20260410.md`
  - `/Users/USER/code/goalcoder/repo_materials/docs/night_shift/NIGHT_SHIFT_VALIDATION_ENTRYPOINT_WORKFLOW.md`

The key pattern is:

- Day Shift locks the slice precisely
- the execution worker follows that locked contract
- the supervisor does not invent missing policy

### Draft contract split

For NSC, the slice file should have two layers:

1. YAML front matter for machine-readable scheduling and launch data
2. Markdown body for worker instructions and human review

This is still only a draft.
The current direction from discussion is to keep the structure very minimal.

### Draft required front-matter fields

These are candidate NSC-required metadata fields for v1:

- `nsc_schema`
- `slice_id`
- `title`
- `depends_on`
- `blocks_parallel_with`
- `exclusive_surfaces`

These are intentionally narrow.
They are the minimum fields NSC appears to need for:

- identity
- dependency order
- parallel-safety rules

Current draft stance:

- `depends_on`, `blocks_parallel_with`, and `exclusive_surfaces` should be
  present even when empty
- branch names, PR names, and test commands should not be required by NSC in
  v1 because those belong to the prompt/repo workflow, not to the supervisor
- if later we discover that launch isolation requires more machine-readable
  fields, we can add them in schema v2

### Field-by-field review

This is the current review of the candidate metadata fields.

#### `nsc_schema`

Keep it.

Purpose:

- allow future schema changes without guesswork
- reject slice files that belong to a different input format

Happy path:

- NSC reads one known schema version and proceeds

Unhappy path:

- missing or unknown schema version should fail fast before scheduling

#### `slice_id`

Keep it.

Purpose:

- stable identity for one slice
- used in logs, status, dependency references, and operator recovery

Happy path:

- unique `slice_id` across the batch

Unhappy path:

- duplicate ids should fail fast
- self-dependency should fail fast

#### `depends_on`

Keep it.

Purpose:

- encode required order explicitly
- encode real launch dependency explicitly

Important clarification:

- if the user requires slice B to happen after slice A, that is exactly what
  `depends_on` is for
- NSC should not infer order from filenames, timestamps, or prompt prose

Happy path:

- empty list means the slice has no declared predecessor
- non-empty list means "do not start this slice until all listed slices reach
  the dependency-unlock state"

Unhappy path and edge cases:

- unknown referenced slice id should fail fast
- dependency cycle should fail fast
- long chains are fine if explicit
- fan-in dependencies are fine if explicit
- if required order was not written down, NSC should not guess it

Important unresolved point:

- we still need to define what exact state satisfies a dependency:
  prompt-reported done, observed merge, or something else
- that cannot be fully locked until later steps on state machine and evidence

Current practical value in v1:

- if the user knows slice B should wait for slice A, `depends_on` gives NSC one
  explicit way to respect that order without guessing
- this is likely the one nontrivial scheduling field still worth keeping in a
  simple v1
- if no order is required, the user can leave it empty and NSC can launch
  slices whenever slots are free

#### `blocks_parallel_with`

Current recommendation: drop it from v1.

Reason:

- it overlaps conceptually with `exclusive_surfaces`
- it adds a second way to express "do not run together"
- it creates symmetry questions:
  - must both slices list each other?
  - if only one side lists the other, is that enough?
- it becomes noisy when many slices conflict with the same shared area

When it would be useful:

- pairwise "never overlap" declarations where there is no obvious shared named
  surface

Why it does not justify itself in v1:

- the same effect can usually be expressed with one shared exclusive surface
  name
- fewer concurrency concepts will make NSC easier to teach

#### `exclusive_surfaces`

Current recommendation: drop it from v1.

Reason:

- the real concern is logical overlap during slice planning
- that should be surfaced in examples and prompt templates so the user or
  planning model thinks about it before launch
- moving that concern into NSC v1 would push the product toward a heavier
  orchestrator before the main execution-supervision problem is solved

Important clarification:

- separate worktrees and branches isolate local execution
- they do not guarantee logical independence
- but teaching users to avoid risky overlap belongs in preparation guidance,
  not in NSC v1 scheduler features

Product implication:

- the examples and docs should explicitly warn about risky overlap
- the examples should show that slices intended for parallel execution should be
  prepared to be obviously independent
- NSC v1 itself should stay out of that policy

### Revised minimal-v1 field recommendation

The current best candidate minimal field set is:

- `nsc_schema`
- `slice_id`
- `depends_on`

Possible optional field:

- `title`

Current recommendation:

- remove `blocks_parallel_with` from v1
- remove `exclusive_surfaces` from v1
- keep `depends_on` as the only nontrivial scheduling field if we want explicit
  ordering support

### Two-sided tradeoff: minimal contract vs richer contract

This is the key design tension for step 2.

#### Side A: keep the slice contract very minimal

Pros:

- lower authoring burden for users
- easier to generate slice files from a model
- fewer ways for the input format to become annoying or brittle
- closer to NSC's real role as a bridge and supervisor rather than a policy
  engine
- easier adoption for users who are new to the Night Shift idea

Cons:

- weaker scheduling guarantees
- more ambiguity inside slice body text
- harder to explain to new users what a "good slice" looks like
- more room for poor Day Shift prep to leak into runtime behavior

#### Side B: require a richer slice contract

Pros:

- stronger preparation discipline
- clearer expectations for the worker
- easier future validation if NSC later grows beyond pure liveness
- better teaching tool for people who do not yet understand Night Shift well

Cons:

- higher authoring burden
- more complicated examples
- easier to overfit NSC into a GoalCoder-like orchestrator
- more temptation for NSC to start enforcing semantics it does not own

### Current recommendation

For NSC v1, prefer the minimal side.

That means:

- keep required front matter as small as possible
- keep the markdown body mostly freeform
- provide examples and templates for good preparation
- teach discipline through examples, not through a heavy required schema

### Draft minimal-v1 contract

Candidate minimal front matter:

- `nsc_schema`
- `slice_id`

Candidate minimal body expectation:

- freeform Night Shift prompt text or slice instructions
- no required headings enforced by NSC
- recommended headings only in examples and docs

This would make NSC scheduling depend on only a few explicit machine-readable
fields while leaving the slice body almost entirely prompt-owned.

### Planning guidance belongs in examples

Because NSC v1 is intentionally minimal, examples must carry part of the
product guidance.

Examples should teach:

- when to split work into separate slices
- when not to run slices in parallel even if NSC technically could
- when to split work into separate batches instead of putting everything into
  one batch folder
- how to prepare slices so parallel work is obviously independent

This guidance belongs in `examples/` and template docs, not in extra v1
metadata fields.

### Draft required body sections

These are candidate recommended markdown sections for examples and guidance:

- `## Objective`
- `## Locked Decisions`
- `## Scope`
- `## Out of Scope`
- `## Stop Boundaries`
- `## Completion / Exit Condition`
- `## Operator Handoff`

What these sections mean:

- `Objective`: the exact thing this slice is trying to accomplish
- `Locked Decisions`: architecture or product-policy decisions already made by
  Day Shift and not open for reinterpretation
- `Scope`: what the worker may change
- `Out of Scope`: nearby tempting work the worker must not absorb
- `Stop Boundaries`: conditions under which the worker should stop instead of
  improvising
- `Completion / Exit Condition`: what the prompt treats as "done enough" for
  this slice
- `Operator Handoff`: what NSC should surface if the run stops and a human must
  resume it

### Why this split fits NSC better than GoalCoder

GoalCoder often includes tests, durable doc updates, review requirements, and
done definitions because the workflow itself owns end-to-end delivery.

NSC does not own that full lifecycle.
So for NSC v1:

- those sections may still appear in the slice body because the worker prompt
  needs them
- but NSC should not require or interpret them as supervisor invariants unless
  we later decide they are needed for launch or recovery

In other words:

- the slice file can be rich
- the NSC-required contract should stay narrow

### Examples and onboarding

Because Night Shift is unfamiliar to many developers, NSC should probably ship
with an `examples/` folder.

Current recommendation:

- `examples/night_shift_prompt_template.md`
- `examples/slices/minimal_independent_slice.md`
- `examples/slices/01_minimal_slice.md`
- `examples/slices/02_minimal_slice.md`
- `examples/batching_notes.md`

Purpose of these examples:

- show the smallest valid slice format
- show deterministic filename ordering for queued slices
- show how to split work into separate batches when real ordering matters
- show how a user can prepare Night Shift input without needing GoalCoder
  background

This is important product work, not just documentation polish.
If the concept is new, examples are part of the interface.

### Current recommendation for step 2

The slice contract should answer exactly three questions:

1. What is this slice?
2. When is it safe to run in parallel or in sequence?
3. What instructions should the worker follow without reopening Day Shift
   decisions?

If a field does not help with one of those three questions, it probably should
not be NSC-required in v1.

Current step-2 direction:

- keep NSC v1 slice metadata minimal
- do not add v1 metadata for dependency or logical-overlap control
- teach overlap thinking through `examples/` and prompt templates
- teach ordering through preparation and batching, not runtime metadata
- keep the slice body mostly prompt-owned and freeform

### Open questions for step 2

1. Should NSC v1 require `title`, or is `slice_id` enough?
2. Should the slice body be entirely freeform in v1, with structure taught
   only through examples?
3. Should NSC ship an `examples/` folder as part of the initial product?
4. Should the user provide a separate Night Shift prompt template file, with
   slice files acting as inserts, or should each slice file be self-contained?

### Step 2

Locked decisions:

- One batch is one folder of slice markdown files.
- One slice is one markdown file.
- NSC v1 should keep the slice contract lean.
- NSC v1 should not enforce runtime dependency logic.
- NSC v1 should not enforce runtime logical-overlap logic.
- If true execution order matters, the user should express that during
  preparation by splitting work into separate batches, not by relying on NSC
  v1 runtime metadata.
- If a batch is given to NSC v1, it is assumed to already contain only slices
  that are intended to run in parallel.
- If a batch contains more than 4 slices, NSC should queue the rest and admit
  them in deterministic filename order as slots free up.
- Queue order should come from deterministic Python filename sorting, not from
  model interpretation of filenames.
- NSC v1 should not require an index file as a second source of truth.
- `nsc_schema` and `slice_id` are the current minimal candidate metadata
  fields.
- The slice body should stay mostly prompt-owned and freeform in v1.
- Examples and templates should teach slice preparation, batching, and overlap
  discipline instead of pushing that policy into NSC v1 metadata.

## Step 3 Draft: Execution Model

This step defines how NSC actually runs one batch.

Now that step 2 is minimal, step 3 becomes the place to lock:

- how NSC discovers slice files in a batch folder
- how NSC orders queued slices
- how NSC creates worktrees
- how NSC launches Codex App Server threads
- how NSC associates one thread with one slice
- how NSC handles slots freeing up
- how NSC distinguishes running, stalled, stopped, and finished threads
- how NSC surfaces recovery context to the operator

### Current starting assumptions

- NSC receives one batch folder containing slice markdown files
- each markdown file is one slice
- queue order is deterministic filename sort
- active concurrency defaults to 4 and may be lowered by CLI
- each running slice gets one separate worktree
- each running slice gets one separate Codex App Server thread
- NSC v1 supervises execution continuity rather than semantic correctness

### Step-3 questions to lock

1. How does NSC decide which markdown files in the batch folder are slices?
   - every `*.md`
   - some filename convention
   - explicit include pattern

2. How should the worker prompt be provided?
   - one separate single-slice prompt template passed by CLI
   - one default template in the repo
   - self-contained slice body only

3. How should NSC create worktree paths?
   - derived from filename stem
   - derived from `slice_id`
   - derived from batch id plus queue position

4. What happens when a slot frees up?
   - immediately launch the next queued slice
   - wait for operator confirmation

5. What exact observable outcomes should NSC record for a thread?
   - `queued`
   - `starting`
   - `running`
   - `stalled`
   - `stopped_unexpectedly`
   - `finished_prompt_owned`

6. What is the minimal information NSC must persist per running slice?
   - slice filename
   - worktree path
   - thread id
   - status

### Current recommendation for step 3

- every `*.md` file in the batch folder should be treated as a slice
- prompt template should be provided separately from the slice folder
- queued slices should be launched automatically as slots free up
- one running slice should always map to one worktree plus one thread
- NSC v1 should keep persisted state minimal and only store what is needed for
  manual operator recovery

### Step 3

Locked decisions:

- NSC v1 should treat every `*.md` file in the batch folder as a slice.
- NSC v1 should use a separate single-slice worker prompt template passed by
  CLI, not the autonomous multi-slice Night Shift prompt.
- Queued slices should be launched automatically as slots free up.
- One running slice should always map to exactly one worktree and exactly one
  Codex App Server thread.
- NSC v1 should keep the default maximum parallel slices hardcoded at `3`.
- NSC v1 should not support NSC restart or NSC-run resume.
- NSC v1 should not persist fields that only exist to support future restart or
  recovery features.
- If NSC persists slice state in v1, it should keep that state minimal and
  focused on manual operator recovery only.

Remaining open questions for step 3:

1. Should worktree naming come from filename stem or from `slice_id`?
2. What exact runtime statuses should NSC record in v1?
3. Should NSC persist minimal per-slice state in a file, or is live display
   plus terminal output enough for v1?

Additional recommendation from later discussion:

- NSC-managed worktrees should live inside the user repository under a hidden
  NSC-owned area, rather than in a global external directory.
- Current preferred default:
  - `.nsc/worktrees/<run_id>/<filename_stem>/`

Reason:

- Codex threads still run in a real git worktree attached to the user repo
- operator discovery is simpler when NSC state stays near the repo
- v1 is easier to understand if all repo-specific NSC state lives under one
  hidden repo-local folder

Important cautions:

- `.nsc/` should be ignored by git
- NSC should not treat files inside `.nsc/` as batch slice inputs
- examples and batch folders should stay outside `.nsc/worktrees/`

### Revised step-3 field recommendation

Rule:

- if NSC v1 does not need a field for live supervision or manual operator
  recovery, drop it

Do not design v1 around future restart or resume.

Do not persist these in v1:

- `concurrency`
- `last_seen_at`
- `run_id` as formal persisted state
- `repo_root` as formal persisted state
- `batch_dir` as formal persisted state
- `prompt_template_path` as formal persisted state

Candidate minimal per-slice persisted fields:

- `slice_file_path`
- `worktree_path`
- `thread_id`
- `status`

These are enough for:

- identifying which slice had a problem
- locating the worktree
- resuming the Codex thread manually
- seeing NSC's last coarse view of that slice

Clarification:

- "persisting" in NSC v1 means writing a tiny on-disk recovery record
- it does not mean supervisor restart support
- it does not mean NSC-run resume support
- it does not mean a full state database or event journal

Current recommendation:

- keep persistence tiny and operator-focused
- persist only enough information so that if NSC itself exits, the operator can
  still find the worktree path, thread id, and last coarse status

Example shape:

```json
{
  "slice_file_path": "slices/01_slice_a.md",
  "worktree_path": ".nsc/worktrees/run123/01_slice_a",
  "thread_id": "abc123",
  "status": "running"
}
```

### Revised step-3 runtime-status recommendation

Recommended minimal runtime statuses for v1:

- `queued`
- `starting`
- `running`
- `attention_needed`
- `finished_prompt_owned`

Reason:

- these are coarse enough to stay honest
- they avoid fake precision about ambiguous situations such as "stalled"
- `attention_needed` is sufficient when NSC notices something clearly off and
  the operator needs to inspect it

### Step 3 review status

Step 3 is now mostly locked.

The current model is:

- batch folder of markdown slices
- deterministic filename ordering
- separate single-slice worker prompt template passed by CLI
- one worktree plus one thread per running slice
- hardcoded maximum parallel slices of `3`
- no NSC restart or NSC-run resume in v1
- tiny on-disk recovery record only

## Step 4 Draft: State Machine

This step should stay aligned with the narrow NSC v1 product boundary.

NSC is not a correctness orchestrator.
So the state machine should describe observable execution lifecycle only.

It should not include states such as:

- `reviewing`
- `ready_to_merge`
- `merged`
- `complete`
- `blocked_dependency`
- `scope_violation`

Those are either:

- planning/preparation concerns
- prompt-owned workflow concerns
- richer future orchestration concerns

### Recommended v1 states

- `queued`
- `starting`
- `running`
- `attention_needed`
- `finished_prompt_owned`

### What each state means

#### `queued`

The slice file was admitted into the batch and is waiting for a free slot.

#### `starting`

NSC is actively creating the worktree and launching the Codex App Server
thread, but does not yet have enough evidence to call the slice fully running.

#### `running`

NSC successfully launched the thread and currently treats the slice as active.

This is an operational statement, not a semantic one.
It means "the run is live enough to continue supervising."

#### `attention_needed`

NSC observed something that requires operator inspection.

Examples:

- thread launch failed
- thread exited unexpectedly
- NSC cannot confidently classify the outcome
- manual inspection is needed before trusting the run

This is intentionally broad.
It avoids fake precision in v1.

#### `finished_prompt_owned`

NSC observed a terminal prompt-owned outcome and the thread is no longer active.

This does not mean:

- code is correct
- tests are sufficient
- merge happened
- durable repo updates are complete

It only means the slice run reached a terminal outcome that NSC is willing to
treat as "finished from the supervisor's point of view."

### Recommended transitions

- `queued -> starting`
  - when NSC assigns the slice to a free slot

- `starting -> running`
  - when worktree creation and thread launch succeed

- `starting -> attention_needed`
  - when launch fails or NSC cannot establish the run cleanly

- `running -> finished_prompt_owned`
  - when NSC observes a terminal prompt-owned outcome

- `running -> attention_needed`
  - when the thread exits unexpectedly or NSC sees something off that it cannot
    classify as a clean finish

### Happy path

- slice enters `queued`
- slot becomes free
- slice moves to `starting`
- thread launches
- slice moves to `running`
- thread reaches terminal prompt-owned outcome
- slice moves to `finished_prompt_owned`

### Unhappy path

- slice enters `queued`
- slot becomes free
- slice moves to `starting`
- launch fails, or thread exits in a way NSC cannot trust
- slice moves to `attention_needed`
- operator uses worktree path and thread id for manual recovery if appropriate

### Edge-case stance

- validation failures for malformed slice files should happen before runtime and
  should be treated as batch admission errors, not runtime states
- NSC v1 should avoid adding separate states for every possible failure mode
- if a situation needs operator eyes and NSC does not have a cheap, reliable
  distinction, prefer `attention_needed`

### Current recommendation for step 4

- keep the state machine operational, not semantic
- use only the 5 coarse states above
- use `attention_needed` as the main catch-all abnormal state
- avoid overfitting the state model to future restart, merge, or correctness
  features

## Step 5 Draft: Deterministic Evidence

This step defines what cheap, concrete facts NSC should verify before changing
runtime state.

Because NSC supervises Codex App Server execution rather than product
correctness, the evidence model should be based on App Server protocol facts
and local execution facts.

It should not be based on:

- merge results
- review status
- tests passing
- repo-specific done criteria

Those remain prompt-owned or repo-owned concerns.

### Locked execution assumption

- NSC v1 should assume one slice run uses one fresh Codex App Server thread.
- NSC v1 should not reuse threads across slices.
- NSC v1 should not treat one resumed or reused thread as covering multiple
  slice runs.

This matches the current GoalCoder-style App Server model and keeps slice
supervision unambiguous.

### App Server facts NSC can rely on

From the read-only GoalCoder integration review, the useful observable facts
are:

- `thread/start` returns a `thread.id`
- `turn/start` returns a `turn.id`
- App Server emits `turn/completed`
- completed turns include a `turn.status`
- terminal prompt-owned content can be captured from the completed turn stream

These are better than guessing from arbitrary worker log prose.

Official confirmation:

- OpenAI's App Server write-up confirms the same lifecycle shape:
  - the client creates a thread and then a turn
  - the server sends progress notifications such as `thread/started` and
    `turn/started`
  - agent message content streams until finalization with `item/completed`
  - the turn ends with `turn/completed`

Source:

- `https://openai.com/index/unlocking-the-codex-harness/`

### Recommended evidence by transition

#### `queued -> starting`

Required evidence:

- NSC selected the slice from the queue
- NSC created or is creating the assigned worktree path

#### `starting -> running`

Required evidence:

- worktree creation succeeded
- `thread/start` succeeded
- `thread.id` exists
- `turn/start` succeeded
- `turn.id` exists

If any of those fail, prefer `starting -> attention_needed`.

#### `running -> finished_prompt_owned`

Required evidence:

- NSC observed `turn/completed` for the active `turn.id`
- completed `turn.status` is `completed`
- NSC captured terminal prompt-owned message content for that turn

This is a supervisor-level finish only.
It does not imply semantic correctness.

Important clarification:

- `finished_prompt_owned` includes prompt-reported success outcomes
- it also includes prompt-reported blocker or stop outcomes
- the key fact is that the turn ended normally and NSC captured the final
  terminal prompt message
- NSC should surface that final message to the operator
- NSC should not deeply reinterpret that message in v1

#### `running -> attention_needed`

Required evidence or triggers:

- App Server process exits unexpectedly
- `turn/completed` arrives with non-`completed` status
- protocol stream is malformed or incomplete
- NSC cannot capture terminal prompt-owned content after completion
- launch succeeded earlier, but the active thread no longer appears trustworthy

### Current recommendation for step 5

- keep evidence cheap and protocol-based
- use App Server lifecycle events, not repo workflow outcomes
- require one fresh thread per slice run
- define "finished" as terminal prompt-owned completion, not semantic success
- distinguish prompt-owned terminal outcomes from transport or protocol failure

### Fallback recommendation for terminal prompt content

If NSC observes `turn/completed` with `turn.status = completed`, it should use
one tiny deterministic fallback chain to capture terminal prompt-owned content.

Recommended priority order:

1. direct final-answer `item/completed`
2. wrapped `codex/event/item_completed`
3. `task_complete.last_agent_message`
4. otherwise `attention_needed`

Rules:

- first matching canonical source wins
- do not merge multiple terminal-looking messages
- do not scrape arbitrary commentary as fallback
- do not inspect repo changes as a proxy for prompt completion

This keeps v1 simple while still avoiding avoidable false alarms.

### Open questions for step 5

1. What exact local signal should count as "App Server process exited
   unexpectedly" in v1?

### Step 5

Locked decisions:

- NSC v1 should assume one slice run uses one fresh Codex App Server thread.
- NSC v1 should not reuse threads across slices.
- NSC v1 should use cheap App Server protocol facts as its main evidence
  source.
- `finished_prompt_owned` means normal App Server completion plus captured final
  terminal prompt-owned content.
- `finished_prompt_owned` may represent prompt-reported success, prompt-reported
  blocker, or prompt-reported stop; NSC should surface the terminal message and
  not reinterpret it deeply in v1.
- `attention_needed` should represent transport, protocol, launch, or terminal
  capture problems that NSC cannot classify cheaply or trust.
- NSC v1 should allow one tiny deterministic fallback chain for terminal prompt
  content:
  - direct final-answer `item/completed`
  - wrapped `codex/event/item_completed`
  - `task_complete.last_agent_message`
  - otherwise `attention_needed`
- NSC v1 should not use broad heuristics such as commentary scraping or repo
  diff inspection to infer prompt completion.

### Step 5 review status

Step 5 is now mostly locked.

## Step 6 Draft: Persistence and Recovery

This step should stay aligned with the narrow NSC v1 product.

NSC v1 does not support:

- supervisor restart
- NSC-run resume
- queued-work restoration
- automatic continuation after NSC itself exits

So step 6 should not become a restart design.

It should define only:

- what minimal recovery breadcrumb NSC leaves behind
- where that breadcrumb lives
- how the operator uses it after something goes wrong

### Current recommendation

#### Persistence purpose

Persistence in NSC v1 exists only for manual operator recovery.

It does not exist for:

- automatic restart
- automatic continuation
- batch replay
- scheduler reconstruction
- prior-run adoption

#### Recovery record shape

Recommended per-slice recovery record:

- `slice_file_path`
- `worktree_path`
- `thread_id`
- `status`

That is enough for:

- identifying the affected slice
- locating its worktree
- resuming the Codex thread manually
- seeing NSC's last coarse view

Do not add richer state in v1 unless a real need appears.

#### Recovery record location

Recommended location:

- `.nsc/recovery/<filename_stem>.json`

This stays separate from:

- `.nsc/worktrees/...`

Reason:

- worktrees are runtime execution areas
- recovery files are operator-facing breadcrumbs
- one recovery folder is easier to inspect than searching inside each worktree

#### Write/update behavior

Recommended behavior:

- create or update the recovery record when a slice enters `starting`
- update it when `thread_id` becomes known
- update it when the coarse `status` changes

Do not build:

- append-only event journals
- sequence counters
- rich lifecycle logs

The file should simply reflect the latest coarse recovery state.

#### Behavior after NSC exits

If NSC itself exits unexpectedly:

- recovery files remain
- worktrees remain
- thread ids remain in recovery files if they were obtained
- operator can inspect `.nsc/recovery/` and decide whether to run
  `codex resume <thread_id>`

NSC should not try to continue from those files automatically in v1.

#### Behavior on a fresh later run

Recommended v1 stance:

- do not silently adopt prior recovery files
- do not auto-reuse prior worktrees
- warning behavior is acceptable
- automatic reuse is not

This keeps recovery explicit and avoids hidden coupling between runs.

#### Cleanup policy

Recommended v1 stance:

- do not auto-delete worktrees
- do not auto-delete recovery files immediately
- leave cleanup explicit for the operator

This is safer for overnight failure cases.

### Happy path

- slice starts
- recovery file is created
- thread id is written once known
- status moves through normal runtime states
- slice finishes with `finished_prompt_owned`
- recovery file reflects that final coarse state

### Unhappy path

- NSC crashes
- terminal closes
- App Server exits
- operator returns later

Still available:

- slice file path
- worktree path
- thread id
- last coarse status

That is enough for manual inspection and possible manual resume.

### Fresh-eye review

This design is intentionally small and consistent with prior decisions:

- it does not smuggle restart support back into v1
- it does not require a database
- it does not require a journal
- it does not require timestamps just to look sophisticated
- it leaves the operator with the exact two things they most need:
  worktree path and thread id

The main product risk here is not conceptual complexity.
It is quiet filesystem drift.

So the testing burden should focus on correctness of `.nsc/recovery/` behavior.

### Testing recommendation

NSC will need focused tests around `.nsc/recovery/` to make sure it behaves
correctly with the rest of the system.

Recommended test coverage:

- recovery file is created when a slice enters `starting`
- recovery file is updated when `thread_id` becomes known
- recovery file is updated when status changes
- recovery files do not land inside `.nsc/worktrees/`
- batch slice discovery ignores `.nsc/recovery/`
- batch slice discovery ignores `.nsc/worktrees/`
- rerunning NSC does not silently adopt prior recovery files
- malformed or partial recovery files do not crash normal batch discovery
- `.nsc/` paths remain outside the normal slice-input scan

### Current recommendation for step 6

- keep persistence recovery-only
- keep recovery files tiny
- keep recovery files repo-local under `.nsc/recovery/`
- keep behavior explicit rather than automatic
- test `.nsc/recovery/` carefully because it becomes part of the product surface

### Open questions for step 6

1. Should recovery filenames use only the filename stem, or filename stem plus
   a short run suffix?
2. Should NSC warn when a new run would overwrite an existing recovery file for
   the same slice filename?

## Step 7 Draft: Merge-Readiness and Landing Policy

This step needs to stay consistent with the boundary already locked for NSC v1.

NSC v1 is not a merge orchestrator.
It is a parallel execution supervisor.

So this step is mostly about defining what NSC does *not* own.

### Current recommendation

#### What NSC v1 should not own

NSC v1 should not own:

- merge-readiness judgment
- review-readiness judgment
- PR or MR quality judgment
- merge execution
- post-merge validation
- durable repo-update validation

Those belong to:

- the prompt
- the repository workflow
- the operator

#### What NSC v1 may surface

NSC may still surface:

- the final prompt-owned terminal message
- the worktree path
- the thread id
- the coarse final runtime state

That means NSC can help the operator see that a slice run ended with:

- prompt-reported success
- prompt-reported blocker
- prompt-reported stop

But NSC should not translate that into:

- "merge-ready"
- "correct"
- "landed"
- "done"

#### Why this is the right v1 boundary

We already decided:

- prompt-owned terminal outcomes are distinct from runtime failures
- NSC should not reinterpret prompt meaning deeply
- NSC should not judge semantic correctness

So a merge policy inside NSC v1 would be a boundary violation.

### Happy path

- prompt runs to terminal completion
- final prompt message says the slice was completed successfully
- NSC marks `finished_prompt_owned`
- operator or existing repo workflow handles any merge or landing action

### Unhappy path

- prompt runs to terminal completion
- final prompt message says the slice is blocked or stopped
- NSC still marks `finished_prompt_owned`
- operator reads the terminal message and decides what to do next

This is not a runtime failure.
It is a prompt-owned terminal outcome.

### Fresh-eye review

The biggest risk in step 7 is accidental product drift.

If NSC starts saying "merge-ready", users will reasonably assume it checked:

- tests
- review
- branch state
- PR state
- merge policy

But we have already decided NSC should not own those things in v1.

So the safest product wording is:

- NSC supervises execution
- prompt/repo/operator own landing

### Current recommendation for step 7

- remove merge-readiness language from NSC v1 product scope
- do not give NSC a `ready_to_merge` state
- do not let NSC claim landing success
- let NSC surface terminal prompt-owned outcomes only

### Open questions for step 7

1. Should NSC v1 display PR/MR or branch references if the final prompt message
   includes them, or should it stay neutral and only surface raw terminal text?
2. Should the concept doc stop using phrases like "executed to the end" and
   replace them with simpler user-facing wording such as "the slice run
   finished normally" or "the slice run ended with a final prompt message"?

### Product-output note

Sooner or later, NSC will need an explicit discussion of what it outputs to the
user.

This is not only an implementation detail.
It is part of the product surface:

- terminal output
- live status view
- any future TUI
- what information NSC highlights when something needs attention

Current recommendation:

- add a dedicated output/TUI discussion later in the process
- do not let UI ideas distort the narrow v1 execution contract
- but do remember that the operator experience is part of the product, not just
  polish

## Step 8 Draft: Unhappy Paths and Operator Actions

This step should define a small set of operator-relevant failure classes and
the required NSC behavior for each.

It should not try to model every theoretical failure.

### Current recommendation

#### 1. Input and batch-admission problems

Examples:

- batch folder missing
- no slice files found
- unreadable markdown file
- malformed minimal metadata
- duplicate `slice_id`
- invalid prompt template path

Recommended behavior:

- fail fast before launching any slice
- print a clear error
- do not partially launch the batch

Reason:

- partial batch admission is confusing
- v1 should stay deterministic and obvious

Important clarification:

- this strict fail-fast rule applies to the initial startup scan
- it should not kill already-running slices just because a new file appears
  later in the folder and is malformed or incomplete

#### 2. Launch problems

Examples:

- worktree creation fails
- App Server startup fails
- `thread/start` fails
- `turn/start` fails

Recommended behavior:

- move slice to `attention_needed`
- write or update the recovery breadcrumb if possible
- continue supervising other slices if the batch still has runnable items

Reason:

- one bad slice should not necessarily stop the whole run
- this matches the main product goal of resilient overnight supervision

#### 3. Runtime problems

Examples:

- App Server process exits unexpectedly
- protocol stream is malformed
- `turn/completed` never arrives cleanly
- terminal prompt message cannot be captured
- thread ends in a way NSC cannot trust

Recommended behavior:

- move slice to `attention_needed`
- preserve worktree path and thread id if known
- surface the reason in simple language
- continue supervising other slices

Reason:

- this is the main operational value of NSC v1

#### 4. Operator-action cases

Examples:

- prompt ended with blocker message
- prompt ended with success message but operator wants to inspect
- recovery files exist from old runs
- operator wants to resume manually
- operator wants to clean up `.nsc/worktrees/` or `.nsc/recovery/`

Recommended behavior:

- surface the relevant paths and ids
- do not auto-resume
- do not auto-clean
- keep operator actions explicit

Reason:

- explicit recovery is part of the v1 philosophy

### Dynamic batch growth during NSC execution

This is a useful v1 addition and does not require heavy engineering if kept
simple.

User mental model:

- NSC starts with an initial set of slice markdown files
- while NSC is running, the user may add more slice markdown files into the
  same batch folder
- NSC should notice them and queue them when slots become available

Recommended v1 behavior:

- do not use a filesystem watcher
- do not build a complex incremental index
- rescan the whole batch folder at simple checkpoints

Recommended checkpoints:

- at startup
- whenever a slot frees up
- optionally on a small polling interval while NSC is running

When rescanning:

- treat new `*.md` files as new slices
- ignore slices already known as active
- ignore slices already marked finished or needing attention
- keep deterministic filename ordering for all not-yet-started slice files

If a newly appeared file is unreadable, malformed, or otherwise invalid during
an in-flight rescan:

- warn
- leave the file alone
- do not stop already-running slices
- try again on the next rescan

If a newly appeared file conflicts with an already known slice identity during
an in-flight rescan:

- warn
- ignore that new conflicting file for the current live run
- keep the current run stable rather than reinterpreting already-admitted work

This is aligned with the simple "add more wood to the fire" model.

### How to mark a slice as no longer pending

This needs care.

One tempting option is to move completed slice files into a `done_slices/`
folder.

Pros:

- simple mental model
- easy to keep the watched folder representing only pending slices

Cons:

- NSC would be modifying user-authored input files
- moving files around can surprise the user
- moving tracked repo files may create unwanted git churn
- it mixes queue-state management with user content management

Fresh-eye recommendation:

- do not move user slice files in v1
- keep user-authored slice files where they are
- use NSC's in-memory known-slice tracking during the live run
- use `.nsc/recovery/` as the durable operator breadcrumb only

That means:

- a finished slice remains in the batch folder
- NSC does not restart it in the same live run because it already knows it
  processed that file
- a fresh future NSC run should be explicit rather than silently inferred from
  leftover files

This is less magical and less intrusive.

### Fresh-eye review

The main risk here is overengineering dynamic queue management.

The simplest working version is:

- rescan the batch folder at simple checkpoints
- treat unseen markdown files as new slices
- do not mutate user slice files
- keep processed/active knowledge in memory for the current live run

That gives you the firepit behavior without building a complex queue database.

### Testing recommendation

Step 8 needs focused tests around dynamic batch growth and file handling:

- new markdown files added during execution are picked up on rescan
- newly discovered slices enter the queue in deterministic filename order
- active slices are not duplicated on rescan
- finished slices are not restarted during the same live run
- `attention_needed` slices are not silently relaunched during the same live
  run
- `.nsc/` folders are ignored during rescans
- batch rescans do not require filesystem watcher support

### Current recommendation for step 8

- keep unhappy-path policy small and operator-relevant
- fail fast on batch-admission problems
- continue unaffected slices when one slice fails at launch or runtime
- keep manual recovery explicit
- support dynamic batch growth by simple batch-folder rescans
- do not move user-authored slice files in v1

### Open questions for step 8

1. Should NSC rescan only when a slot frees up, or also on a short polling
   interval while slots are still full?
2. On a fresh future NSC run, should existing finished slice files require the
   operator to move them out manually, or should NSC later gain an explicit
   archive/cleanup command?

### Step 8

Locked decisions:

- keep unhappy-path policy small and operator-relevant
- fail fast on initial batch-admission problems
- continue unaffected slices when one slice fails at launch or runtime
- keep manual recovery explicit
- support dynamic batch growth by simple batch-folder rescans
- do not move user-authored slice files in v1
- initial startup scan may fail fast on malformed input
- in-flight rescans should warn on malformed new files instead of killing
  already-running slices
- `.nsc/` paths must remain outside normal slice-input rescans

### Step 8 review status

Step 8 is now mostly locked.

## Step 9 Draft: Smallest Safe v1

This step should synthesize all earlier decisions into the narrowest version of
NSC that is still worth building.

Current recommendation:

- repo-local tool
- batch folder of markdown slice files
- separate single-slice prompt template passed by CLI
- hardcoded max parallel slices of `3`
- one worktree plus one fresh Codex App Server thread per running slice
- deterministic filename ordering
- simple rescans so new slice files added during execution can join the queue
- coarse runtime states only
- tiny recovery breadcrumb only
- no NSC restart or NSC-run resume
- no dependency graph
- no logical-overlap scheduler
- no merge-readiness or landing ownership
- no movement of user-authored slice files

The output of this step should be one short explicit v1 scope and one short
explicit out-of-scope list.

### Proposed v1 scope

NSC v1 is a repo-local parallel Night Shift execution supervisor for markdown
slice batches.

In scope:

- one batch folder of markdown slice files
- one separate single-slice worker prompt template passed by CLI
- deterministic filename ordering
- hardcoded maximum parallel slices of `3`
- one separate git worktree per running slice
- one fresh Codex App Server thread per running slice
- coarse runtime state tracking:
  - `queued`
  - `starting`
  - `running`
  - `attention_needed`
  - `finished_prompt_owned`
- tiny `.nsc/recovery/` breadcrumb files for manual operator recovery
- simple batch-folder rescans so newly added markdown slices can join the queue
- operator-visible reporting of:
  - slice file path
  - worktree path
  - thread id
  - coarse runtime state
  - final prompt-owned terminal message when captured

### Proposed v1 out of scope

- dependency graphs
- runtime ordering semantics
- logical-overlap scheduling
- merge-readiness judgment
- merge execution
- review-readiness judgment
- post-merge validation
- semantic correctness judgment
- NSC restart support
- NSC-run resume support
- auto-restart of stopped runs
- automatic reuse of prior worktrees or recovery files
- movement of user-authored slice files
- index files as a second source of truth
- heavy state persistence
- filesystem watcher complexity

### Fresh-eye review

This still looks like the smallest version that solves the original problem:

- long Night Shift runs stopping unexpectedly
- desire to run several prepared slices in parallel
- desire to preserve manual recovery context without building a larger
  orchestrator

It also still avoids the biggest v1 traps:

- turning NSC into a planner
- turning NSC into a merge manager
- turning NSC into a restart/recovery engine
- turning NSC into a scheduler that pretends to understand semantics

### Current recommendation for step 9

- lock the v1 around execution supervision, not correctness
- keep queue behavior simple and filename-driven
- keep recovery manual
- keep input and operator model easy to explain

### Step 9

Locked decisions:

- NSC v1 should be a repo-local parallel Night Shift execution supervisor for
  markdown slice batches.
- NSC v1 scope should stay focused on execution supervision, simple queueing,
  separate worktrees, App Server threads, and manual recovery breadcrumbs.
- NSC v1 should explicitly exclude dependency graphs, overlap scheduling,
  merge-readiness, landing ownership, restart/resume support, and movement of
  user-authored slice files.

### Step 9 review status

Step 9 is now mostly locked.

## Step 10 Draft: Implementation Shape

This step is not the implementation plan itself.

It is the final design pass that translates the locked product decisions into a
clean implementation shape.

Goal:

- make the system easy to build
- make it easy to test
- avoid accidental complexity
- avoid baking speculative future features into the first code layout

### Layman terms overview

In simple terms, NSC v1 should work like this:

- you point NSC at a folder of slice markdown files
- you point NSC at one single-slice Night Shift prompt template
- NSC starts up to 3 slice runs at a time
- each run gets its own worktree and its own Codex App Server thread
- NSC keeps an eye on those runs
- if one finishes normally, NSC shows the final prompt message
- if one has trouble, NSC marks it as needing attention and shows the thread id
  and worktree path
- if you add more markdown slice files to the folder, NSC notices them later
  and adds them to the queue

That is the whole product.
It is a live supervisor, not a planning engine and not a merge manager.

### High-level implementation overview

At a high level, NSC v1 probably needs these parts:

1. CLI entrypoint
   - start one NSC run
   - accept the slice folder path
   - accept the prompt template path

2. Batch scanner
   - list `*.md` files in the batch folder
   - ignore `.nsc/`
   - sort deterministically by filename
   - identify new files on rescan

3. Worktree manager
   - create one worktree per slice under `.nsc/worktrees/...`
   - avoid collisions with existing worktree paths

4. App Server runner
   - start one fresh Codex App Server thread per slice
   - provide the single-slice prompt input
   - capture thread id, turn id, and terminal message content

5. Runtime supervisor
   - track coarse slice states
   - react when slots free up
   - rescan the batch folder for new slices
   - move slices to `attention_needed` when needed

6. Recovery writer
   - write tiny `.nsc/recovery/*.json` breadcrumbs

7. Output layer
   - print operator-meaningful status
   - surface final prompt messages
   - surface recovery information clearly

### Developer-facing description

For a developer, the key implementation challenge is not sophisticated
reasoning.
It is disciplined boundaries.

The code should make it hard to accidentally add:

- dependency scheduling
- overlap scheduling
- merge ownership
- restart/resume machinery
- rich state persistence

So the implementation shape should follow the locked product shape closely.

A good v1 codebase likely has:

- one small CLI module
- one batch-discovery module
- one worktree module
- one App Server runner module
- one supervisor loop module
- one recovery-file module
- one output/status rendering module

This is better than one large orchestrator file because:

- tests stay focused
- product boundaries stay visible
- later features can be added intentionally rather than by accretion

### Decisions to think through before implementation planning

These are the remaining implementation-shape decisions worth making before the
actual implementation plan.

#### 1. CLI shape

Recommendation:

- start with one command, likely `nsc run`

Inputs:

- slice folder path
- prompt template path

Pros:

- simple
- easy to explain
- aligned with v1 scope

Cons:

- future cleanup or inspect commands would come later

Recommended option:

- one primary `run` command only for v1

#### 2. How to construct the final prompt sent to Codex

Options:

- built-in default prompt template plus slice file content appended or
  interpolated
- external prompt template provided by CLI plus slice file content appended or
  interpolated
- fully self-contained slice file only

Pros of a built-in default template:

- better first-run experience
- teaches the intended NSC single-slice model
- reduces the chance that users reuse the wrong autonomous Night Shift prompt
- keeps NSC usable out of the box

Cons:

- prompt text becomes part of the product surface
- prompt revisions become product changes

Pros of template-based construction:

- matches the locked decision that NSC uses a separate single-slice prompt
  template
- easier to teach and evolve the worker shell prompt
- slice files can stay simple

Cons:

- requires a clear composition rule

Recommended option:

- NSC should ship with one default opinionated single-slice prompt template
- CLI should allow overriding that template
- prompt assembly should stay simple and deterministic

#### 3. Worktree naming rule

Options:

- filename stem
- `slice_id`

Pros of filename stem:

- matches queue ordering and file-centric v1 model
- easier to debug manually
- no need to parse file content for path naming

Cons:

- relies on filenames staying sensible

Recommended option:

- filename stem under `.nsc/worktrees/<run_scope>/...`

#### 4. Recovery record format

Options:

- one JSON file per slice
- one combined JSON file for the whole run

Pros of one file per slice:

- simpler updates
- easier operator inspection
- failures are localized

Cons:

- more small files

Recommended option:

- one small JSON file per slice under `.nsc/recovery/`

#### 5. Rescan strategy

Options:

- rescan only when a slot frees up
- rescan on a simple polling interval too

Pros of slot-free rescan only:

- simplest
- fewer moving parts
- likely enough for v1

Cons:

- if all 3 slots are long-running, newly added slices are not noticed until
  something finishes

Pros of adding a small polling interval:

- more responsive to newly added files

Cons:

- more implementation detail
- more chances for noisy edge cases

Recommended option:

- start with rescan when a slot frees up
- optionally add a very small polling loop only if the implementation stays
  trivial

#### 6. Output model

Options:

- plain line-oriented terminal output only
- lightweight live table or TUI

Pros of plain output:

- fastest to build
- easiest to test
- easiest to keep reliable

Cons:

- less polished operator experience

Recommended option:

- plain terminal output first
- defer TUI until after the core loop proves useful

#### 7. Test shape

Recommendation:

- bias hard toward focused unit tests plus a few integration-style loop tests

Important areas:

- batch scanning
- `.nsc/` exclusion
- deterministic ordering
- worktree creation path logic
- recovery file creation and update
- state transitions
- dynamic file addition during execution
- App Server completion vs attention-needed classification

Pros:

- high confidence in the product contract
- catches filesystem and queue mistakes early

Cons:

- requires a little more discipline up front

Recommended option:

- focused test suite around the supervisor loop and filesystem behavior before
  UI polish

### Fresh-eye review

The biggest implementation-shape risk now is not missing a subsystem.
It is choosing a structure that quietly invites future feature creep.

So the aligned recommendation is:

- one narrow `run` flow
- file-centric batch scanning
- filename-driven queueing
- one fresh thread per slice
- tiny recovery files
- plain operator output
- focused tests around filesystem and runtime-state behavior

### Current recommendation for step 10

- implementation shape should mirror the locked product shape closely
- keep modules small and responsibility-driven
- prefer file-centric, operator-transparent behavior over abstraction-heavy
  design
- defer TUI, restart, dependency scheduling, and landing integration
  intentionally
