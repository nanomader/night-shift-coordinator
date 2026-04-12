# NSC Prompt Night Shift Rewrite Plan

## Objective

Rewrite the built-in `NSC_PROMPT.md` so the default NSC worker prompt follows a
single-slice Night Shift flow closer to GoalCoder:

- perform a short repo-context pre-plan checkpoint
- use `$plan-author-review-loop`
- use `$implementation-new-branch-pr`
- poll GitHub Codex review after the PR is opened

The rewrite must stay inside the NSC v1 product boundary.

## Current Gap / Problem Statement

The current built-in prompt in [NSC_PROMPT.md](/Users/USER/code/night-shift-coordinator/NSC_PROMPT.md)
is intentionally narrow, but it is too generic for the workflow you now want.

Current issues:

1. it does not tell the worker to run a pre-plan checkpoint before planning
2. it does not explicitly direct the worker to use
   `$plan-author-review-loop`
3. it does not explicitly direct the worker to use
   `$implementation-new-branch-pr`
4. it does not include a GitHub Codex review polling step after PR creation
5. it does not define a clean single-slice stop boundary after the PR review
   loop

That makes the built-in prompt weaker than the GoalCoder Night Shift execution
pattern, even though NSC should remain single-slice and repo-agnostic.

## Locked Decisions

- Keep the NSC worker prompt single-slice only.
- Do not add any serial Night Shift loop that selects another slice after the
  current slice.
- Do not add merge ownership, landing ownership, or queue ownership to the
  prompt.
- Reuse the repo-local workflow pattern where available:
  pre-plan checkpoint, plan, implementation on a new branch/PR, then review.
- Prefer existing skills over ad hoc prose when the workflow depends on them.
- Keep the built-in prompt repo-agnostic.
- The prompt may mention GitHub Codex review polling, but it must treat missing
  repo support or missing review tooling as a blocker to report, not something
  to invent.
- `$implementation-new-branch-pr` already opens the PR and posts the required
  head-scoped `@codex review` comment, so the next step is polling that review
  outcome rather than posting a duplicate request immediately.

## Scope

### In scope

- rewrite [NSC_PROMPT.md](/Users/USER/code/night-shift-coordinator/NSC_PROMPT.md)
- make the built-in prompt explicitly mention:
  - repo-local docs reading
  - pre-plan checkpoint
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
  - GitHub Codex review polling
  - single-slice stop boundary and final-report shape
- update docs that describe the built-in prompt if they become inaccurate

### Out of scope

- changing NSC runtime code
- changing CLI behavior
- adding new prompt templating logic
- adding merge automation
- adding serial multi-slice continuation
- changing review-poller code or GitHub integration code

## Implementation Steps With Concrete Files

1. Rewrite [NSC_PROMPT.md](/Users/USER/code/night-shift-coordinator/NSC_PROMPT.md)
   - Preserve the single-slice framing.
   - Add a clear ordered workflow:
     1. read assigned slice file
     2. read repo-local instructions
     3. run the pre-plan checkpoint from repo context
     4. use `$plan-author-review-loop`
     5. use `$implementation-new-branch-pr`
     6. poll GitHub Codex review on the opened PR head, for example with
        `$pr-codex-review-poll`
     7. if confirmed findings above LOW remain inside the slice, fix them on the
        same branch/PR and repeat the bounded review step once
     8. otherwise stop with one final summary
   - Make the stop boundary explicit:
     - do not select another slice
     - do not merge
     - do not widen scope beyond the assigned slice

2. Align prompt wording with the GoalCoder pattern while stripping serial-lane
   behavior
   - Keep strong instructions for:
     - exact-slice scope
     - blocker reporting
     - repo-truth checking before planning
   - Remove or avoid:
     - repeat-after-merge instructions
     - canonical queue selection
     - durable-state rules that require repo-specific policy files

3. Update documentation if needed
   - [README.md](/Users/USER/code/night-shift-coordinator/README.md)
   - [night_shift_coordinator_concept.md](/Users/USER/code/night-shift-coordinator/night_shift_coordinator_concept.md)
   - Only change text that describes the built-in prompt behavior.

## Tests

### Manual checks

- read `NSC_PROMPT.md` end to end and verify it still describes exactly one
  slice run
- verify the prompt mentions both required skills exactly:
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
- verify the prompt includes a GitHub Codex review polling step
- verify the prompt does not immediately post a second `@codex review` request
  after `$implementation-new-branch-pr`
- verify the prompt does not instruct merge or next-slice continuation

### Local validation

- no code tests are required if only docs/prompt text change
- if any code or parser assumptions are touched unexpectedly, run:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`

## Risks + Mitigations

- Risk: the rewrite drifts back into full serial Night Shift behavior.
  - Mitigation: keep “one slice only, then stop” as a repeated explicit rule.

- Risk: the prompt becomes GoalCoder-specific instead of repo-agnostic.
  - Mitigation: reuse workflow shape, not GoalCoder lane-specific file lists or
    queue contracts.

- Risk: adding review polling language implies runtime capabilities NSC itself
  does not own.
  - Mitigation: frame polling as worker behavior inside the prompt, not NSC
    runtime orchestration.

- Risk: the prompt references unavailable tooling in some repos.
  - Mitigation: require the worker to stop and report missing workflow support
    instead of improvising replacements.

## Done Definition

The work is done when all of the following are true:

- [NSC_PROMPT.md](/Users/USER/code/night-shift-coordinator/NSC_PROMPT.md) clearly describes a
  single-slice Night Shift execution flow
- the prompt explicitly includes the pre-plan checkpoint
- the prompt explicitly instructs the worker to use:
  - `$plan-author-review-loop`
  - `$implementation-new-branch-pr`
- the prompt explicitly includes a GitHub Codex review polling step
- the prompt treats `$implementation-new-branch-pr` as the source of the first
  review request and does not immediately require a duplicate review comment
- the prompt explicitly forbids selecting another slice or merging as part of
  the same run
- any prompt-related wording in
  [README.md](/Users/USER/code/night-shift-coordinator/README.md) or
  [night_shift_coordinator_concept.md](/Users/USER/code/night-shift-coordinator/night_shift_coordinator_concept.md)
  remains accurate
