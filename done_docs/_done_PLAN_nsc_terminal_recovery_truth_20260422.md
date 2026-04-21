## Objective

Make NSC terminal completion state explicit for operators after a slice finishes,
without changing scheduling semantics or the built-in prompt contract.

## Current Gap/Problem Statement

Today recovery JSON records only `status`, optional `progress`, and optional
`progress_updated_at`. When a slice reaches `NSC_PROGRESS: WAITING_FOR_MERGE`
and then later finishes normally, the recovery file can still show
`progress: waiting_for_merge` alongside `status: finished_prompt_owned`.
That is technically consistent with the current implementation, but it is
operator-ambiguous because recovery does not also persist the parsed
`NSC_RESULT` markers, merged SHA, PR number, or a completion timestamp.

The terminal output also prints the raw final message, but it does not render a
small normalized result summary that makes merge completion obvious at a glance.

## Locked Decisions

- Keep `NSC_RESULT:` lines as the only authoritative ordered-unlock contract.
- Keep `NSC_PROGRESS:` and `NSC_EVENT:` informational only.
- Do not add restart, repair, or merge-management behavior.
- Do not change the built-in prompt contract in `NSC_PROMPT.md`.
- Preserve `progress` as the last observed informational progress marker rather
  than changing scheduler semantics around it.
- Improve operator clarity by adding explicit terminal metadata rather than by
  inferring completion from stale progress values.

## Scope

In:

- extend recovery records for terminal slices with parsed terminal-result
  metadata and completion time
- persist that metadata when `FinishedPromptOwned` is handled
- render a compact normalized result line in terminal output for finished slices
- update tests and README to match the refined recovery/output contract

Out:

- any change to scheduling order, unlock policy, or merge gating
- any change to `NSC_PROMPT.md` marker vocabulary
- any auto-resume, auto-retry, or post-merge cleanup feature
- any dynamic refill or run-resume behavior

## Implementation Steps With Concrete Files

1. Extend the recovery model in `src/recovery.rs`.
   - Add optional terminal fields to `RecoveryRecord`:
     - `result`
     - `pr_number`
     - `merged_main_sha`
     - `completed_at`
   - Reuse parsed terminal markers rather than inventing a second completion
     truth.

2. Refine terminal-result parsing support in `src/terminal_result.rs`.
   - Make the parsed terminal result kind serializable so it can be written into
     recovery directly.
   - Keep existing parse rules unchanged.

3. Persist terminal metadata in `src/supervisor.rs`.
   - In `handle_terminal_result`, parse `terminal_message` with
     `parse_terminal_result_markers`.
   - On `FinishedPromptOwned`, write recovery with:
     - `status = finished_prompt_owned`
     - existing last-seen `progress`
     - existing `progress_updated_at`
     - parsed `result` metadata when present
     - `completed_at` generated at completion time
   - Thread the parsed terminal metadata into
     `OutputEvent::SliceFinishedNormally` so output rendering uses the same
     normalized facts rather than reparsing or inferring later.
   - Keep attention-needed writes unchanged unless a small consistency helper
     makes the implementation clearer.

4. Improve operator output in `src/output.rs`.
   - Extend `OutputEvent::SliceFinishedNormally` with optional normalized
     terminal metadata from `src/supervisor.rs`.
   - For `SliceFinishedNormally`, render a short normalized summary line when
     that metadata is present, for example merged/manual-followup/blocked.
   - Keep the raw final message visible so operators still have the exact prompt
     output.

5. Update and extend tests.
   - `tests/recovery.rs`: assert the expanded recovery JSON shape for a
     finished slice with terminal metadata.
   - `tests/terminal_result.rs`: cover any serialization additions for the
     terminal result kind if needed.
   - `tests/execution_truth.rs` and/or `tests/supervisor_loop.rs`: assert that
     finished slices persist parsed terminal metadata in recovery and pass that
     metadata through the finished output event.
   - `src/output.rs` unit tests: assert the compact normalized result line
     appears for finished slices with markers while the raw final message stays
     visible.

6. Update docs in `README.md`.
   - Expand the recovery-file section to state that:
     - `progress` is only the last observed informational progress marker
     - terminal truth comes from the finished status plus optional parsed
       result metadata
   - Update runtime output examples or operator-facing meanings if the new
     normalized result line is user-visible.

7. Archive and history follow-through after implementation.
   - Move the completed plan into `done_docs/` with `_done` prefix.
   - Append one sentence to `TASKS_HISTORY.md` naming the landed change and
     plan file.

## Tests

- `cargo test --test recovery`
- `cargo test --test terminal_result`
- `cargo test --test execution_truth`
- `cargo test --test supervisor_loop`
- `cargo test`
- Manual spot-check:
  - inspect one finished recovery JSON fixture/output path and confirm terminal
    metadata makes completion obvious even when `progress` remains
    `waiting_for_merge`

## Risks + Mitigations

- Risk: introducing a second source of truth for completion.
  - Mitigation: persist only data derived from `parse_terminal_result_markers`
    and keep `NSC_RESULT:` as the canonical unlock contract.

- Risk: overloading recovery with verbose terminal text.
  - Mitigation: store only normalized terminal metadata, not the whole final
    message.

- Risk: changing operator output too aggressively.
  - Mitigation: add one compact summary line while preserving the existing raw
    final-message line.

- Risk: docs drift from the refined recovery contract.
  - Mitigation: update `README.md` in the same change and cover the new shape
    in tests.

## Done Definition

- Finished slice recovery JSON can explicitly answer whether the slice merged or
  ended in another terminal result without relying on stale progress markers.
- Recovery JSON includes completion metadata for finished prompt-owned runs.
- Terminal output shows a compact normalized result line for finished slices
  when terminal markers are present.
- Existing ordered-unlock behavior still depends on parsed `NSC_RESULT:` lines
  and remains unchanged.
- Tests cover the new recovery/output behavior and pass.
- `README.md` documents the refined recovery/output contract.
