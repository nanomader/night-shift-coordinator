# NSC Wave-Dirs Scheduling Decision Memo

## Status

Proposed next scheduling extension for NSC.

This memo does not change the current shipped meaning of:

```text
nsc run <folder>
```

Default `nsc run` remains the flat parallel-safe batch model.

## Objective

Add one explicit scheduling mode for repositories such as GoalCoder that
prepare Night Shift work as an ordered root directory containing durable child
batch directories.

The mode should let one operator point NSC at the parent root and trust NSC to:

- preserve Day Shift's locked sibling-directory order
- allow safe parallelism inside one prepared child directory
- stop before the next child directory if the current one did not land
  successfully enough

Boundary sentence:

**NSC executes prepared Night Shift work. It does not prepare, repair, or
reinterpret that work.**

## Current Gap

Current NSC behavior supports only:

- one batch = one folder
- one slice = one markdown file
- all slices in that folder are assumed parallel-safe
- up to 3 slices run at once
- extra files queue in lexicographic order

That is enough for independent slices, but it is not enough for GoalCoder's
prepared Wave 2 PHP queue, where later prepared batch folders explicitly depend
on earlier folders landing on `main`.

## Proposed Operator Surface

Add one explicit scheduling policy:

```text
nsc run <folder> --schedule wave-dirs
```

Meaning:

- the supplied folder is a wave root
- each immediate child directory is one wave-dir
- wave-dirs run in locked lexicographic order
- only one wave-dir runs at a time
- within one wave-dir, current NSC flat-batch behavior stays in effect

Default `nsc run <folder>` without `--schedule wave-dirs` remains unchanged.

## GoalCoder Fit

This policy matches GoalCoder's prepared layout directly.

Example root:

```text
repo_materials/night_shift_batches/validation_entrypoint_wave2_php/
```

Example child wave-dirs:

- `batch02_php_vendor_bin_phpunit/`
- `batch03_composer_test_phpunit/`
- `batch04_composer_phpunit_phpunit/`
- `batch05_vendor_bin_pest/`
- `batch06_composer_test_pest/`
- `batch07_composer_validate_strict/`
- `batch08_composer_audit/`

This preserves the human-reviewed Day Shift structure instead of flattening the
queue into one filename namespace.

## Locked Decisions

### 1. Default mode does not change

`nsc run <folder>` keeps the current flat-batch semantics.

### 2. One explicit ordered-root policy

Use:

```text
nsc run <folder> --schedule wave-dirs
```

No DAG inference.
No metadata schema.
No dynamic refill in this mode.

### 3. Immediate child directories are the only wave units

In `--schedule wave-dirs` mode:

- only immediate child directories count as waves
- one sibling directory = one wave-dir
- sibling wave-dirs run strictly one at a time
- there is no nested wave discovery in v1

### 4. Strict wave-dir naming grammar

Wave-dir names must match:

```text
^batch[0-9]{2}_[a-z0-9._-]+$
```

Rules:

- numeric batch ids must be unique
- order is ascending by batch id
- numbering must be contiguous within the supplied root
- the root does not need to begin at `batch01_*`

Examples:

- valid:
  - `batch02_php_vendor_bin_phpunit`
  - `batch03_composer_test_phpunit`
- invalid:
  - `02_php_vendor_bin_phpunit`
  - `batch3_php_vendor_bin_phpunit`
  - mixed `batch02_*` plus `notes/`

### 5. Allowed non-directory entries in the wave root

Allowed:

- `README.md`
- hidden entries such as dotfiles or `.DS_Store`

Everything else at the wave-root level should fail fast.

Reason:

- GoalCoder's current prepared root already contains `README.md`
- silently ignoring arbitrary extra root files is too loose for this mode

### 6. Discovery freezes at startup

For `--schedule wave-dirs` mode in v1:

- discover the full set of immediate child wave-dirs once at startup
- do not admit newly added sibling wave-dirs mid-run
- discover slice files inside each child wave-dir when that wave-dir begins
- require the prepared wave-root and slice files to already exist in the
  repository state NSC executes from
- do not copy slice files into worktrees to compensate for incomplete prepared
  input

This keeps ordered execution deterministic.

### 7. Current flat-batch behavior still applies inside one wave-dir

Within one child wave-dir:

- discover non-empty `*.md` slice files
- run up to max parallelism
- supervise them with normal NSC behavior

But the operational meaning must be stronger than "parallel-safe."

All slices inside one wave-dir must be:

- safe to run in parallel
- safe to review in parallel
- safe to merge and land from the same fetched `origin/main` base

If one slice inside a child wave-dir depends on another slice in that same
directory landing first, they do not belong in the same wave-dir.

### 8. Base refresh happens between wave-dirs, not inside one wave-dir

NSC should:

- fetch `origin/main` before starting each new wave-dir
- create all slice worktrees for that wave-dir from that same fetched base
- avoid mutating the user's main checkout

NSC should not fetch a new base between slices inside the same wave-dir.

### 9. Unlock requires machine-readable prompt output plus remote verification

`NSC_RESULT: MERGED_SUCCESS` is necessary, but not sufficient on its own.

Each slice in the current wave-dir must emit exact standalone terminal lines:

```text
NSC_RESULT: MERGED_SUCCESS
NSC_MERGED_MAIN_SHA: <40-hex-sha>
NSC_PR: <number>
```

Other exact non-success result markers:

```text
NSC_RESULT: MANUAL_FOLLOWUP
NSC_RESULT: BLOCKED
NSC_RESULT: STOPPED
```

Unlock rule for the next wave-dir:

1. every slice in the current wave-dir emitted `NSC_RESULT: MERGED_SUCCESS`
2. every slice emitted `NSC_MERGED_MAIN_SHA`
3. NSC fetches `origin/main`
4. for each emitted merged-main SHA, NSC verifies:

```bash
git merge-base --is-ancestor <merged_main_sha> origin/main
```

Continue only if all checks pass.

Otherwise:

- stop immediately
- do not start the next sibling wave-dir

### 10. Prompt compatibility is required for wave-dirs mode

The built-in prompt must emit the exact `NSC_RESULT:` and
`NSC_MERGED_MAIN_SHA:` lines required by this mode.

If the operator uses `--prompt-template` together with `--schedule wave-dirs`,
that prompt must satisfy the same output contract.

Otherwise NSC cannot unlock the next wave-dir safely.

## Success Definition

For `--schedule wave-dirs`, a child wave-dir is successful only if:

- every slice in that wave-dir reports `NSC_RESULT: MERGED_SUCCESS`
- every slice reports `NSC_MERGED_MAIN_SHA`
- the fetched `origin/main` contains all emitted merged-main SHAs

"PR opened" is not enough.
"Worker finished talking" is not enough.
"Review clear but not merged" is not enough.

GoalCoder compatibility requires landed remote truth before the next wave-dir
starts.

## Stop Conditions

NSC must stop and refuse to unlock the next wave-dir if any of these happen in
the current wave-dir:

- `NSC_RESULT: MANUAL_FOLLOWUP`
- `NSC_RESULT: BLOCKED`
- `NSC_RESULT: STOPPED`
- missing required machine-readable result lines
- failed `origin/main` refresh
- emitted merged-main SHA is not contained in fetched `origin/main`
- one slice in the current wave-dir does not complete successfully enough

## Operator Output

Ordered progression should be explicit.

Minimum useful output:

```text
Found 7 wave directories
Starting wave-dir 1/7: batch02_php_vendor_bin_phpunit
Wave-dir 1/7 finished with MERGED_SUCCESS
Starting wave-dir 2/7: batch03_composer_test_phpunit
Wave-dir 2/7 stopped: MANUAL_FOLLOWUP
```

The operator should never have to guess whether NSC is still inside the current
wave-dir or has unlocked the next one.

## Non-Goals

This mode does not ask NSC to:

- infer dependencies automatically
- build a DAG scheduler
- reinterpret default flat-batch behavior
- admit new sibling wave-dirs mid-run
- implement automatic retry policy
- infer merged success from natural-language prose alone

## Recommended Next Step

If this memo is accepted, the next implementation plan should cover:

1. CLI addition for `--schedule wave-dirs`
2. wave-root discovery and strict validation
3. per-wave-dir execution loop
4. machine-readable result parsing
5. merged-remote verification against fetched `origin/main`
6. updated built-in prompt contract
7. README and concept-doc updates
