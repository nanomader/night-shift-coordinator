# Objective

Add simple slice progress tracking to NSC using prompt-emitted `NSC_PROGRESS:` markers, and remove the current noisy debug heartbeat output.

# Current Gap / Problem Statement

NSC's `debug` mode currently tells the operator only that a slice is `starting`, `running`, and then emits periodic `still running (60s elapsed)` heartbeat lines until completion. That proves the supervisor is alive, but it does not tell the operator what phase the slice is actually in, and the heartbeat becomes noisy on longer runs.

The built-in prompt now follows a fairly fixed workflow: planning, implementation, review wait, optional review-fix loop, and merge wait. NSC can surface that usefully without becoming a GoalCoder-like orchestrator if the built-in prompt emits explicit machine-readable progress markers.

# Locked Decisions

- Progress tracking is prompt-driven, not inferred from GitHub state, file changes, or raw App Server event heuristics.
- The built-in prompt will emit exact standalone progress lines in this fixed set:
  - `NSC_PROGRESS: PLANNING`
  - `NSC_PROGRESS: IMPLEMENTING`
  - `NSC_PROGRESS: WAITING_FOR_REVIEW`
  - `NSC_PROGRESS: FIXING_REVIEW_FINDINGS`
  - `NSC_PROGRESS: WAITING_FOR_MERGE`
- Progress markers are informational only.
- `NSC_PROGRESS:` must not affect wave-dir unlocks, success state, or scheduling.
- Only `NSC_RESULT:` continues to control wave progression and stop behavior.
- Progress lines should appear in `debug` mode only.
- The current periodic `still running` heartbeat should be removed from `debug` mode.
- NSC should persist the latest known progress phase into recovery JSON.
- If multiple progress markers appear in one assistant message, NSC should keep the last recognized phase from that message.
- NSC should only emit a debug progress line when the phase actually changes.
- If a prompt emits no progress markers, NSC should stay quiet about phases and keep normal existing runtime behavior.

# Scope

In:

- `NSC_PROMPT.md`
- `src/terminal_result.rs` or a nearby parser surface for progress markers
- `src/app_server.rs` if terminal outcome parsing needs to carry progress information
- `src/supervisor.rs`
- `src/output.rs`
- `src/recovery.rs`
- tests covering marker parsing, debug output, and recovery JSON shape
- README and concept-doc updates describing progress visibility
- task history and archived plan as part of the implementation workflow

Out:

- progress inference from GitHub/API/tool events
- new scheduling states or unlock rules
- transition validation/state-machine enforcement for progress phases
- normal `run` mode output expansion
- new CLI flags

# Implementation Steps With Concrete Files

1. Update `NSC_PROMPT.md`.
   - Instruct the built-in prompt to emit exact standalone `NSC_PROGRESS:` lines when the slice enters each coarse phase.
   - Keep the existing `NSC_RESULT:` contract unchanged.
   - Keep the progress contract explicitly informational.

2. Add progress-marker parsing.
   - Extend `src/terminal_result.rs` or a nearby parsing module with a parser for `NSC_PROGRESS:` lines.
   - Support only the fixed allowed phase set.
   - If multiple recognized progress lines appear in one message, keep the last one.

3. Thread progress information through runtime handling.
   - Update `src/app_server.rs` and/or `src/supervisor.rs` so NSC can observe progress markers from assistant output during the run, not only at final completion.
   - Keep the implementation scoped to existing terminal-message/event handling instead of inventing a separate channel.

4. Update debug output behavior in `src/output.rs`.
   - Add a phase-change output event such as `phase: planning`.
   - Remove the current `SliceStillRunning` heartbeat rendering from debug mode.
   - Keep `starting`, `running`, phase changes, and final outcomes.

5. Update recovery persistence in `src/recovery.rs`.
   - Add an optional `progress` field storing the latest known phase.
   - Update writes from `src/supervisor.rs` when progress changes.
   - Preserve compatibility for runs that have no progress markers.

6. Update tests.
   - Add parser tests for valid progress markers, repeated markers, and mixed content.
   - Update output tests to assert that debug mode shows phase changes and no longer relies on heartbeat lines.
   - Update recovery tests for the optional `progress` field.
   - Add one supervisor-style test if needed to prove the latest phase is retained and persisted.

7. Update docs.
   - `README.md`: explain in simple English that `debug` mode can now show coarse slice phases when the prompt emits `NSC_PROGRESS:` markers.
   - `night_shift_coordinator_concept.md`: record the progress-marker contract and the removal of the heartbeat-style debug noise.

# Tests

Unit / local tests:

- parser tests for `NSC_PROGRESS:` markers
- `cargo test --test terminal_result`
- `cargo test --test recovery`
- `cargo test --test prompt_template`

Full validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`

Manual verification:

- inspect `NSC_PROMPT.md` for exact `NSC_PROGRESS:` lines
- run `nsc debug ...` in a suitable repo and confirm phase changes appear without periodic `still running` heartbeat spam
- confirm recovery JSON contains the latest phase when available

# Risks + Mitigations

- Risk: carrying progress through the runtime could accidentally broaden NSC into a second orchestration layer.
  - Mitigation: keep progress markers prompt-emitted, coarse, and informational only.

- Risk: prompt emits repeated or noisy progress markers.
  - Mitigation: keep only recognized markers, store the last one from a message, and print only on actual phase change.

- Risk: custom prompts without `NSC_PROGRESS:` markers could create confusing empty output.
  - Mitigation: keep the feature optional outside the built-in prompt and avoid warning spam.

- Risk: removing the heartbeat could reduce confidence that the process is alive when no progress markers appear.
  - Mitigation: keep `starting` and `running` lines, and rely on phase changes for the built-in prompt path.

# Done Definition

The change is done when all of the following are true:

- The built-in prompt emits exact `NSC_PROGRESS:` lines for the fixed phase set.
- NSC can parse and surface those phase changes in `debug` mode.
- `debug` mode no longer prints the periodic `still running` heartbeat.
- Progress remains informational only and does not affect result handling or scheduling.
- Recovery JSON can store the latest known progress phase.
- README and concept docs describe the new progress visibility accurately.
- Full Rust validation passes locally.
- The plan is archived in `done_docs/`, task history is updated, the branch is pushed, and a PR is opened with the required Codex review comment.
