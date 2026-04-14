# Objective

Make the built-in NSC prompt retry the GitHub Codex review request once on the same unchanged PR head before stopping for missing review response, while shortening the first wait window to 10 minutes.

# Current Gap / Problem Statement

The built-in prompt currently polls every 60 seconds for up to 20 minutes per PR head, then stops with `NSC_RESULT: MANUAL_FOLLOWUP` if the clean GitHub Codex review comment is still missing. That is better than the older vague contract, but it still gives up after a single request path.

GoalCoder's Night Shift workflow has one stronger rule that worked in practice: after the first bounded wait window is exhausted with no accepted head-scoped outcome, it posts exactly one second `@codex review` request on the same unchanged head and reruns the poll loop once.

You want to reuse that part of the GoalCoder behavior, but with a shorter first window: 10 minutes.

# Locked Decisions

- This is a prompt/documentation/test change, not a Rust runtime change.
- The built-in prompt should keep polling every 60 seconds.
- The first review wait window should be 10 minutes total for the current PR head.
- If no accepted head-scoped outcome appears in that first 10-minute window, the prompt should post exactly one second `@codex review` request on the same unchanged PR head.
- After that one retry request, the prompt should run one second bounded poll loop on the same unchanged head.
- The prompt should not keep retrying indefinitely.
- `MANUAL_FOLLOWUP` for missing review response should only happen after:
  - first 10-minute window exhausted, and
  - exactly one same-head retry posted, and
  - second 10-minute window exhausted without an accepted outcome.
- Keep the merge-success gate unchanged:
  - merge only after a GitHub Codex review comment on the current PR head begins with `Codex Review: Didn't find any major issues.`
- Keep the current exact `NSC_RESULT` marker contract unchanged.

# Scope

In:

- `NSC_PROMPT.md`
- `README.md`
- `night_shift_coordinator_concept.md`
- `tests/prompt_template.rs`
- task history and archived plan as part of the implementation workflow

Out:

- Rust code changes in `src/`
- new machine-readable result kinds such as `missing_after_retry`
- adopting GoalCoder's full durable fallback-policy system
- changes to review parser behavior

# Implementation Steps With Concrete Files

1. Update `NSC_PROMPT.md` review-polling workflow.
   - Change the first wait window from 20 minutes to 10 minutes.
   - Keep the 60-second poll interval.
   - Add the explicit same-head retry rule after the first exhausted window.

2. Update the merge-gate language in `NSC_PROMPT.md`.
   - State that pending/no clean comment yet remains a bounded waiting state.
   - State that after the first 10-minute window expires with no accepted outcome, the model must post exactly one second `@codex review` request on the same unchanged head.
   - State that only after the second 10-minute window also expires may the model stop with `NSC_RESULT: MANUAL_FOLLOWUP`.

3. Keep the final-message requirements in `NSC_PROMPT.md` aligned with the retry path.
   - When stopping due to missing review response after retry, require the final message to report:
     - total wait time used
     - last poll status
     - PR number
     - current PR head SHA
     - that one same-head retry was posted

4. Update `README.md`.
   - Replace the current 20-minute wording with the new 10-minute + one same-head retry contract.
   - Keep the README concise and user-facing.

5. Update `night_shift_coordinator_concept.md`.
   - Keep the concept doc aligned with the same 10-minute + one retry review gate.

6. Update `tests/prompt_template.rs`.
   - Replace the current `20 minutes` expectation with `10 minutes`.
   - Add an assertion that the built-in prompt contains the same-head retry rule.
   - Keep the existing merge-gate phrase assertion.

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

- inspect `NSC_PROMPT.md` and confirm:
  - first poll window is 10 minutes
  - interval is 60 seconds
  - exactly one same-head retry is required before manual follow-up
- inspect `README.md` and `night_shift_coordinator_concept.md` for the same contract

# Risks + Mitigations

- Risk: shortening the window too much could increase retry frequency unnecessarily.
  - Mitigation: retain the same 60-second polling interval and allow one full second 10-minute window after the retry.

- Risk: the prompt/docs drift apart again.
  - Mitigation: update prompt, README, concept doc, and the prompt-template test together.

- Risk: the prompt becomes ambiguous about whether the retry must be on the same head.
  - Mitigation: state `same unchanged PR head` explicitly in all affected text.

# Done Definition

The change is done when all of the following are true:

- `NSC_PROMPT.md` explicitly uses a 10-minute first poll window with 60-second polling.
- `NSC_PROMPT.md` explicitly requires exactly one second `@codex review` request on the same unchanged PR head before giving up on missing review response.
- `NSC_PROMPT.md` only allows `MANUAL_FOLLOWUP` for missing review after both bounded windows are exhausted.
- `README.md` and `night_shift_coordinator_concept.md` document the same contract.
- `tests/prompt_template.rs` verifies the updated built-in prompt contract.
- Full Rust validation passes locally.
- The plan is archived in `done_docs/`, task history is updated, the branch is pushed, and a PR is opened with the required Codex review comment.
