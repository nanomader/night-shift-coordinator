# Objective

Set NSC's Codex App Server runtime to use `gpt-5.4` with reasoning effort `medium` by default, and make that default explicit and testable.

# Current Gap / Problem Statement

NSC currently launches `codex app-server` and starts threads/turns without setting a model or reasoning effort in `src/app_server.rs`. That leaves runtime behavior up to whatever defaults the local Codex environment chooses. The product requirement is now explicit: NSC should use `gpt-5.4` and reasoning `medium`.

The nearby code gap is also testability. `tests/app_server_adapter.rs` only checks a debug representation and does not assert the JSON-RPC payload shape for `thread/start` and `turn/start`.

# Locked Decisions

- NSC must default to model `gpt-5.4`.
- NSC must default to reasoning effort `medium`.
- Keep the current CLI surface unchanged; no new user-facing flags in this change.
- Keep the change scoped to the App Server adapter path and directly related docs/tests.
- Use the Codex App Server request contract already supported by the vendored schema and local developer guide:
  - `thread/start` may set `model`
  - `turn/start` may set `model` and `effort`
- Do not add prompt-level workarounds for model selection.

# Scope

In:

- `src/app_server.rs`
- nearby adapter tests in `tests/app_server_adapter.rs`
- any small helper refactor needed to make request payloads directly testable
- README/runtime docs that describe the default model behavior
- task history + archived plan as part of the implementation workflow

Out:

- new CLI flags for model or reasoning overrides
- changing prompt content beyond doc references if needed
- broader scheduling/runtime refactors outside the App Server adapter path
- summary/reasoning-summary tuning unless required by schema correctness

# Implementation Steps With Concrete Files

1. Inspect `src/app_server.rs` and define a single source of truth for the default model and reasoning effort.
   - Add explicit constants such as `DEFAULT_MODEL` and `DEFAULT_REASONING_EFFORT`.
   - Keep them near the App Server request-building logic so the policy is easy to find.

2. Refactor request construction in `src/app_server.rs` into small helpers that return JSON payloads for:
   - `thread/start`
   - `turn/start`
   This keeps the wire contract testable without needing a live app-server process.

3. Update `thread/start` request construction in `src/app_server.rs`.
   - Include `model: "gpt-5.4"` in the `thread/start` params.
   - Preserve existing `approvalPolicy`, `cwd`, `ephemeral`, and `serviceName` behavior.

4. Update `turn/start` request construction in `src/app_server.rs`.
   - Include `model: "gpt-5.4"`.
   - Include `effort: "medium"`.
   - Preserve existing `approvalPolicy`, `cwd`, `input`, and `threadId` behavior.

5. Add focused adapter tests in `tests/app_server_adapter.rs`.
   - Assert the thread-start payload includes the default model.
   - Assert the turn-start payload includes the default model and `medium` effort.
   - Assert the prompt text logic still includes the repo-relative slice path and immutable-run instruction.
   - Keep tests deterministic and local-only.

6. Update `README.md`.
   - State that NSC defaults Codex App Server runs to `gpt-5.4` with reasoning effort `medium`.
   - Keep the wording factual and limited to runtime defaults.

7. Update `TASKS_HISTORY.md` and archive the plan to `done_docs/` during implementation.

# Tests

Unit / local tests:

- `cargo test --test app_server_adapter`
- any inline unit tests added to `src/app_server.rs`

Full validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`

Manual verification:

- inspect the resulting request-building helpers and confirm they encode:
  - `thread/start` -> `model: gpt-5.4`
  - `turn/start` -> `model: gpt-5.4`, `effort: medium`

# Risks + Mitigations

- Risk: set the fields on the wrong RPC method or with wrong keys.
  - Mitigation: use the vendored Codex schema / local developer guide as the contract, and pin the exact payload shape in tests.

- Risk: broaden the change into a user-configurable model system.
  - Mitigation: keep this change default-only and scoped to the adapter.

- Risk: weaken existing runtime behavior while refactoring request construction.
  - Mitigation: preserve all existing fields in the helpers and test the stable prompt text shape.

# Done Definition

The change is done when all of the following are true:

- NSC no longer relies on implicit Codex defaults for model/reasoning.
- `src/app_server.rs` explicitly sends `gpt-5.4` and `medium` through the supported App Server request fields.
- Adapter tests verify the exact payload shape for the new defaults.
- README documents the runtime defaults accurately.
- Full Rust validation passes locally.
- The plan is archived in `done_docs/`, task history is updated, the branch is pushed, and a PR is opened with the required Codex review comment.
