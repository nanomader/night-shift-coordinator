# Objective

Make NSC's built-in prompt wait longer for the GitHub Codex review gate before stopping with `NSC_RESULT: MANUAL_FOLLOWUP`, and document that policy explicitly.

# Current Gap / Problem Statement

The current built-in prompt in `NSC_PROMPT.md` requires polling the GitHub Codex review outcome and merging only after a clean head-scoped comment begins with `Codex Review: Didn't find any major issues.`. But it does not define a concrete poll window or interval.

In real use, this caused a wave-dirs run to stop even though the clean GitHub Codex comment appeared about 9 minutes after the review request. The prompt likely gave up too early and emitted `NSC_RESULT: MANUAL_FOLLOWUP` while the review was still pending.

Nearby docs in `README.md` and `night_shift_coordinator_concept.md` describe the review polling as bounded, but they do not state the actual policy.

# Locked Decisions

- This is a prompt and documentation change, not a Rust runtime/scheduling change.
- The built-in prompt should poll GitHub Codex review for up to 20 minutes total.
- The built-in prompt should poll every 60 seconds.
- The same 20-minute / 60-second policy should apply after a same-slice follow-up push for a new PR head.
- The prompt should distinguish between:
  - review findings above LOW
  - clean no-findings review state
  - pending/no clean comment yet
- `NSC_RESULT: MANUAL_FOLLOWUP` should be emitted for review delay only after the full polling window is exhausted.
- When the prompt stops due to exhausted review wait time, the final message should report:
  - total wait time used
  - last observed review state
  - PR number
  - current PR head SHA

# Scope

In:

- `NSC_PROMPT.md`
- `README.md`
- `night_shift_coordinator_concept.md`
- `tests/prompt_template.rs`
- task history and archived plan as part of the implementation workflow

Out:

- Rust code changes in `src/`
- changes to `src/terminal_result.rs` marker parsing
- new CLI flags or runtime configuration
- changes to the merge-success marker contract

# Implementation Steps With Concrete Files

1. Update `NSC_PROMPT.md` review-polling workflow language.
   - Replace the vague bounded polling wording with an explicit contract:
     - poll every 60 seconds
     - keep polling for up to 20 minutes total
     - after same-slice follow-up fixes on a new head, apply the same polling window again for that new head
   - Keep the existing merge gate phrase unchanged.

2. Tighten the prompt's stop behavior in `NSC_PROMPT.md`.
   - State that `MANUAL_FOLLOWUP` for review delay is allowed only after the full 20-minute window expires without the required clean comment.
   - State that pending/no-clean-comment-yet is not itself a merge success and not itself a blocker; it is a bounded waiting state.

3. Expand the final-message instructions in `NSC_PROMPT.md`.
   - Require the model to report total wait time, last poll status, PR number, and current PR head SHA when stopping due to review delay.
   - Keep the existing `NSC_RESULT` standalone marker lines unchanged.

4. Update `README.md`.
   - Replace vague “bounded GitHub Codex review polling” wording with the explicit 20-minute / 60-second policy.
   - Keep the README focused on the runtime contract rather than internal prompt prose.

5. Update `night_shift_coordinator_concept.md`.
   - Reflect the same explicit review-polling contract so the concept doc remains aligned with the built-in prompt.

6. Update `tests/prompt_template.rs`.
   - Add assertions that the built-in prompt contains:
     - `20 minutes`
     - `60 seconds`
     - language indicating that `MANUAL_FOLLOWUP` is only for exhaustion of the review wait window, not immediate pending state

# Tests

Unit / local tests:

- `cargo test --test prompt_template`

Full validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`

Manual verification:

- inspect `NSC_PROMPT.md` and confirm the explicit review polling policy is present
- inspect docs and confirm the same 20-minute / 60-second rule appears in user-facing language

# Risks + Mitigations

- Risk: wording drifts and the prompt/docs disagree again.
  - Mitigation: update `NSC_PROMPT.md`, `README.md`, and `night_shift_coordinator_concept.md` together in one change.

- Risk: the prompt becomes ambiguous about pending review versus real blockers.
  - Mitigation: explicitly define pending review as a bounded waiting state and reserve `MANUAL_FOLLOWUP` for exhausted waiting.

- Risk: tests remain too weak and only assert the old merge gate phrase.
  - Mitigation: add prompt-template assertions for the polling window and stop condition text.

# Done Definition

The change is done when all of the following are true:

- `NSC_PROMPT.md` explicitly says to poll every 60 seconds for up to 20 minutes before stopping for missing clean review.
- The prompt applies the same policy after a same-slice follow-up push on a new PR head.
- The prompt requires extra final-message context when manual follow-up is caused by exhausted review waiting.
- `README.md` and `night_shift_coordinator_concept.md` document the same policy.
- `tests/prompt_template.rs` verifies the new built-in prompt contract.
- Full Rust validation passes locally.
- The plan is archived in `done_docs/`, task history is updated, the branch is pushed, and a PR is opened with the required Codex review comment.
