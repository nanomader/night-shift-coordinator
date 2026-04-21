## Objective

Route malformed terminal-result markers from finished prompt-owned slice runs
through NSC's attention-needed path so unattended runs keep trustworthy
terminal breadcrumbs instead of stale running recovery state.

## Current Gap/Problem Statement

`handle_terminal_result` now parses terminal markers at the start of the
`FinishedPromptOwned` branch. If `parse_terminal_result_markers` returns
`InvalidTerminalResult`, the `?` exits early before recovery is updated or
`SliceNeedsAttention` is emitted. That leaves the slice with stale `running`
recovery data even though NSC already received a prompt-owned terminal message.

This weakens `.nsc/recovery` exactly in the malformed-marker case that should
be easiest for operators to diagnose after an unattended run.

## Locked Decisions

- Keep `NSC_RESULT:` lines as the only authoritative ordered-unlock contract.
- Keep `verify_wave_dir_unlock` fail-closed on malformed terminal markers.
- Do not soften merge-gate or ordered-unlock behavior.
- Treat malformed finished-turn markers as slice-scoped attention-needed state
  at the supervisor seam instead of a raw run error.
- Persist `completed_at` for that terminal attention state because NSC has
  already reached a slice terminal message.
- Keep the fix scoped to supervisor/recovery/output/tests/docs only.

## Scope

In:

- `handle_terminal_result` error handling for malformed markers in the
  `FinishedPromptOwned` path
- recovery breadcrumbs for that terminal attention state
- explicit test coverage for malformed finished-turn markers
- small doc clarification if the operator contract needs it

Out:

- any change to parser validation rules
- any change to wave-dir or wave-root unlock semantics
- any restart, retry, or auto-repair behavior
- any prompt-contract vocabulary change

## Implementation Steps With Concrete Files

1. Refine `src/supervisor.rs`.
   - Replace the early `?` in the `FinishedPromptOwned` branch with explicit
     handling of `parse_terminal_result_markers`.
   - For `Ok(markers)`, keep the existing finished path unchanged.
   - For `Err(NscError::InvalidTerminalResult(...))`, write recovery with:
     - `status = attention_needed`
     - last-seen `progress`
     - last-seen `progress_updated_at`
     - no parsed terminal metadata
     - `completed_at = now`
   - Emit `OutputEvent::SliceNeedsAttention` with a reason that explicitly says
     `invalid terminal result markers: ...`.
   - Return `CompletedSliceOutcome::AttentionNeeded` for that slice so ordered
     scheduling stops through the normal attention path.

2. Keep `src/terminal_result.rs` unchanged unless a tiny helper is needed for
   clearer branching in `src/supervisor.rs`.

3. Confirm `write_recovery` behavior in `src/supervisor.rs` still maps the new
   attention path cleanly and does not accidentally persist stale terminal
   metadata from a parse failure.

4. Add regression coverage.
   - `tests/supervisor_loop.rs` or `tests/execution_truth.rs`:
     - feed a `FinishedPromptOwned` terminal message with malformed
       `NSC_PR` or `NSC_MERGED_MAIN_SHA`
     - assert the run surfaces `attention:<label>` rather than a raw
       `run-stopped:*`
     - assert the recovery file ends in `status = attention_needed`
     - assert `completed_at` is present
     - assert `result`, `pr_number`, and `merged_main_sha` are absent
   - Keep existing parser tests in `tests/terminal_result.rs` as the canonical
     malformed-marker validation rail.

5. Update docs if needed.
   - `README.md`: if the new attention reason or terminal breadcrumb behavior
     is user-visible enough to matter, add one short clarification that
     malformed finished-turn markers become `needs attention` rather than a
     silent stale-running recovery state.

6. Archive and history follow-through after implementation.
   - Move the completed plan into `done_docs/` with `_done` prefix.
   - Append one sentence to `TASKS_HISTORY.md` naming the malformed-marker
     attention hardening and this plan file.

## Tests

- `cargo test --test terminal_result --test supervisor_loop --test execution_truth`
- `cargo test`
- Manual logic check:
  - verify the malformed-marker path still leaves `verify_wave_dir_unlock`
    fail-closed because the slice outcome is now attention-needed rather than a
    false merged success

## Risks + Mitigations

- Risk: accidentally weakening malformed-marker fail-closed behavior.
  - Mitigation: keep `verify_wave_dir_unlock` unchanged and route the slice to
    attention-needed, which still blocks the next wave unit.

- Risk: writing misleading terminal metadata on parse failure.
  - Mitigation: persist no parsed result fields when marker parsing fails.

- Risk: broadening the error handling too much and swallowing unrelated bugs.
  - Mitigation: target only `InvalidTerminalResult` from the finished-turn
    parse seam; keep other error branches unchanged.

## Done Definition

- A finished prompt-owned turn with malformed terminal markers is recorded as
  slice attention-needed instead of leaving stale running recovery state.
- Recovery for that case includes `completed_at` and excludes parsed terminal
  metadata.
- Output reports `needs attention` with an explicit invalid-marker reason.
- Ordered scheduling still stops fail-closed through the existing
  attention-needed path.
- Regression tests cover the malformed finished-turn marker path and pass.
