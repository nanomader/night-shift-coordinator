# Objective

Add a small `NSC_EVENT:` debug marker family for high-value review and merge milestones, and persist one timestamp in recovery JSON so operators can tell when the latest progress/activity update happened.

# Current Gap / Problem Statement

NSC now shows coarse `NSC_PROGRESS:` phase changes in `debug` mode and no longer prints the old elapsed-time heartbeat. That is a better baseline, but long runs still have quiet periods that are hard to interpret.

The most important missing visibility is not another generic phase. It is explicit workflow milestones around review polling, same-head retry, follow-up push, and merge start. Those are the moments operators actually care about when a run appears idle.

Recovery JSON now stores the latest optional `progress` phase, but it does not tell the operator when that phase was last updated. A stopped run with `waiting_for_review` is more useful if NSC also records when that state was last observed.

# Locked Decisions

- Keep the existing `NSC_PROGRESS:` phase set unchanged.
- Do not restore any heartbeat or elapsed-time ticker.
- Add one new machine-readable marker family: `NSC_EVENT:`.
- Support only this fixed event set in the built-in prompt:
  - `NSC_EVENT: REVIEW_POLLING_STARTED`
  - `NSC_EVENT: REVIEW_RETRY_POSTED`
  - `NSC_EVENT: FOLLOWUP_PUSHED`
  - `NSC_EVENT: MERGE_STARTED`
- `NSC_EVENT:` markers are informational only.
- `NSC_EVENT:` must not affect scheduling, wave-dir unlocks, or success handling.
- Only `NSC_RESULT:` remains authoritative for progression and stop behavior.
- Surface event lines only in `nsc debug ...`.
- Keep the implementation prompt-driven; do not infer review/merge state in Rust from GitHub or file changes.
- Persist one optional recovery timestamp for the latest progress or event update.
- Use a stable UTC RFC 3339 timestamp string so it is human-readable and deterministic.

# Scope

In:

- `NSC_PROMPT.md`
- `src/terminal_result.rs`
- `src/app_server.rs`
- `src/supervisor.rs`
- `src/output.rs`
- `src/recovery.rs`
- tests covering marker parsing, debug output, and recovery JSON
- `README.md` and `night_shift_coordinator_concept.md`
- task history and archived plan as part of the implementation workflow

Out:

- restoring heartbeat output
- adding more phase names
- making Rust infer workflow events from GitHub APIs or worktree changes
- changing scheduling semantics or unlock rules
- adding event-driven warnings in normal `run` mode

# Implementation Steps With Concrete Files

1. Extend the marker model in `src/terminal_result.rs`.
   - Add a fixed event enum for the four supported `NSC_EVENT:` values.
   - Add an event-marker parser similar to `parse_progress_marker()`.
   - Keep the same “last recognized marker wins within one message” rule.

2. Thread debug events through the App Server adapter in `src/app_server.rs`.
   - Replace the progress-only side channel with one small run-update channel carrying either progress phases or debug events.
   - Continue extracting only from completed agent messages tied to the active turn.
   - Parse both `NSC_PROGRESS:` and `NSC_EVENT:` from the same message text without changing terminal result handling.

3. Surface events in the supervisor and terminal output.
   - Update `src/supervisor.rs` to persist the latest progress state and latest activity timestamp when either a phase or event marker arrives.
   - Update `src/output.rs` with one debug-only output event for workflow milestones, for example `event: review polling started`.
   - Keep event output deduplicated only by exact receipt order; do not add a state machine.

4. Extend recovery persistence in `src/recovery.rs`.
   - Add one optional `progress_updated_at` field storing the latest progress/activity timestamp as RFC 3339 UTC.
   - Preserve compatibility when no progress or event markers were emitted.

5. Update the built-in prompt in `NSC_PROMPT.md`.
   - Emit `NSC_EVENT: REVIEW_POLLING_STARTED` before each review-polling window begins.
   - Emit `NSC_EVENT: REVIEW_RETRY_POSTED` immediately after posting the same-head retry.
   - Emit `NSC_EVENT: FOLLOWUP_PUSHED` after pushing same-slice review fixes.
   - Emit `NSC_EVENT: MERGE_STARTED` immediately before merge begins.
   - Keep the existing `NSC_PROGRESS:` and `NSC_RESULT:` contracts unchanged otherwise.

6. Update tests.
   - Add parser tests for valid event markers and mixed progress/event messages.
   - Update App Server tests to prove event extraction from assistant messages.
   - Update output tests to assert debug event rendering.
   - Update recovery tests for optional `progress_updated_at`.
   - Add or extend one supervisor-style test to prove a received event updates the recovery timestamp and debug output.

7. Update docs.
   - `README.md`: explain the new debug event lines in simple operator language.
   - `night_shift_coordinator_concept.md`: record that debug visibility includes explicit review/merge events and an activity timestamp in recovery.

# Tests

Focused tests:

- `cargo test --test terminal_result`
- `cargo test --test recovery`
- `cargo test --test prompt_template`
- `cargo test --test execution_truth`

Full validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`

Manual verification:

- inspect `NSC_PROMPT.md` for exact `NSC_EVENT:` lines
- run `nsc debug ...` in a suitable repo and confirm event lines appear without any heartbeat
- inspect one `.nsc/recovery/...json` file and confirm `progress_updated_at` appears when progress or event markers were emitted

# Risks + Mitigations

- Risk: the new event family starts to blur the line between NSC and a richer orchestrator.
  - Mitigation: keep the event set fixed, prompt-driven, and informational only.

- Risk: prompt messages could emit both progress and event markers in the same assistant message.
  - Mitigation: parse both marker families from the same text and treat them as separate updates.

- Risk: recovery timestamps could become noisy or unstable across tests.
  - Mitigation: use RFC 3339 UTC strings and assert only presence/format where exact time control is unnecessary.

- Risk: custom prompts without `NSC_EVENT:` markers remain quieter than the built-in prompt.
  - Mitigation: keep the feature optional outside the built-in prompt and avoid warning spam.

# Done Definition

The change is done when all of the following are true:

- the built-in prompt emits the four exact `NSC_EVENT:` lines at the intended review/merge milestones
- NSC can parse and surface those events in `nsc debug ...`
- no heartbeat output exists in source or docs
- events remain informational only and do not affect scheduling or result handling
- recovery JSON can store optional `progress_updated_at`
- README and concept docs describe the new debug visibility accurately
- full Rust validation passes locally
- the plan is archived in `done_docs/`, task history is updated, the branch is pushed, and a PR is opened with the required Codex review comment
