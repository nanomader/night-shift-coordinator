# NSC Running Slice Staleness Boundary Fix Plan

## 1. objective

Remove the nearby stale-running failure mode in NSC by making silent
app-server stalls fail closed into `attention_needed`, keeping recovery
timestamps truthful, and aligning recovery metadata semantics with what the
README currently claims.

## 2. current gap/problem statement

- A slice can remain in recovery state `running` forever after `start_run()`
  succeeds if the Codex child stays alive but stops producing protocol output.
- [`src/supervisor.rs`](src/supervisor.rs) waits only for progress/event
  updates or completion-channel delivery; it has no watchdog for a started
  slice that becomes silent after launch.
- [`src/app_server.rs`](src/app_server.rs) applies only the broad
  eight-hour run timeout; it does not distinguish a healthy long-running turn
  from a child that stopped emitting protocol traffic.
- `progress_updated_at` is currently bumped on workflow events as well as
  progress markers, even though the documented field meaning is “last observed
  informational progress marker.”
- Terminal attention paths do not always persist `completed_at`, which weakens
  operator-visible recovery breadcrumbs.

## 3. locked decisions

- Canonical source of truth for app-server silence detection remains
  [`src/app_server.rs`](src/app_server.rs), not the supervisor loop.
- The fix stays within NSC’s existing runtime-supervision boundary. No restart,
  retry, or repair behavior is added.
- Ordered scheduling semantics stay unchanged: a slice that goes silent becomes
  `attention_needed`, which keeps later wave-dirs or wave-roots locked.
- Recovery JSON remains intentionally small. This plan does not add a new
  recovery schema field if current docs can be made true by tightening write
  behavior instead.
- `progress` stays informational only. The fix clarifies timestamp truth rather
  than expanding scheduling meaning.

## 4. scope

In scope:

- add an explicit app-server protocol-silence watchdog for started turns
- convert silence expiry into `TerminalRunOutcome::AttentionNeeded` with an
  operator-readable reason
- ensure the supervisor writes `completed_at` for all terminal attention
  outcomes
- stop updating `progress_updated_at` on pure workflow events when the progress
  phase itself did not change
- update the nearby docs and tests that describe recovery/runtime behavior

Out of scope:

- changing flat, wave-dirs, or wave-roots scheduling rules
- adding dynamic discovery, restart, retry, or auto-resume behavior
- broad recovery-schema expansion beyond what this fix strictly needs
- prompt-contract changes in [`NSC_PROMPT.md`](NSC_PROMPT.md)

## 5. implementation steps with concrete files

1. In [`src/app_server.rs`](src/app_server.rs), add a bounded “no protocol
   traffic observed” timeout around the per-notification wait path used after a
   turn starts.
   - keep the existing eight-hour total run timeout
   - add a separate silence timeout that resets every time NSC receives a new
     protocol payload while waiting for the running turn
   - return `TerminalRunOutcome::AttentionNeeded` with a precise reason when
     the silence window expires
   - structure the timeout wrapper so unit tests can exercise it with short
     injected durations instead of the production constant

2. In [`src/supervisor.rs`](src/supervisor.rs), tighten recovery timestamp
   behavior.
   - update `progress_updated_at` only when a new progress phase is observed
   - keep workflow events visible in debug output without making recovery imply
     fresh progress
   - set `completed_at` for every terminal `attention_needed` write path,
     including app-server, protocol, and generic terminal failures

3. Keep the recovery contract coherent in [`src/recovery.rs`](src/recovery.rs)
   and [`README.md`](README.md).
   - if no new field is needed, confirm the current JSON shape still matches
     the tightened semantics
   - update runtime-output and recovery docs so silence timeout and terminal
     attention timestamps are explicit

4. Add direct unit coverage in [`src/app_server.rs`](src/app_server.rs) tests.
   - test the existing broad run timeout still maps to `AttentionNeeded`
   - add a focused silence-timeout test for the new helper or wait wrapper
   - keep the test deterministic with short synthetic futures or paused Tokio
     time rather than live app-server execution

5. Add caller-level execution-truth coverage in
   [`tests/execution_truth.rs`](tests/execution_truth.rs).
   - add one test proving a started slice with no completion and no updates
     eventually becomes `attention_needed` instead of hanging forever
   - add one test proving workflow events alone do not refresh
     `progress_updated_at` for an unchanged progress phase
   - update or extend the existing recovery assertions so terminal attention
     outcomes require `completed_at`

## 6. tests

Direct/unit:

- `cargo test app_server -- --nocapture`
- focused unit coverage in [`src/app_server.rs`](src/app_server.rs) for the
  new silence-timeout helper or wrapper

Caller-level smoke / regression:

- `cargo test execution_truth -- --nocapture`
- specifically verify:
  - silent started runs stop as `attention_needed`
  - malformed-marker attention still records `completed_at`
  - event-only activity does not advance `progress_updated_at`

Broader local validation before commit/push:

- `cargo test`
- `cargo fmt --check`

## 7. risks + mitigations

- Risk: a silence timeout that is too aggressive could classify a slow but
  still-healthy turn as attention-needed.
  Mitigation: keep the watchdog separate from the total run timeout, choose a
  conservative duration, and document that it measures missing protocol traffic
  rather than total run length.

- Risk: fixing the hang by adding new recovery fields would widen the contract
  more than necessary.
  Mitigation: prefer tightening existing field semantics first and only add a
  field if the current schema cannot truthfully express the needed state.

- Risk: event/progress timestamp tightening could accidentally drop useful
  debug visibility.
  Mitigation: keep debug output unchanged and limit the change to recovery JSON
  timestamp writes.

## 8. done definition

- A started slice that goes silent at the app-server protocol layer no longer
  remains indefinitely `running`; it becomes `attention_needed` with a specific
  reason.
- Recovery JSON for all terminal attention outcomes includes `completed_at`.
- `progress_updated_at` reflects last observed progress-marker change, not pure
  workflow events.
- Unit tests cover the new silence-timeout behavior.
- Execution-truth tests cover the started-but-silent hang regression and the
  tightened recovery semantics.
- [`README.md`](README.md) accurately documents the nearby runtime-output and
  recovery behavior after the change.
