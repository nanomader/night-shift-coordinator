# Objective

Make Night Shift Coordinator request Codex fast service tier explicitly instead
of relying on ambient Codex profile/config inheritance.

# Current Gap / Problem Statement

NSC currently hard-codes the Codex model and reasoning effort in
`src/app_server.rs`, but it does not explicitly request fast service tier when
it launches `codex app-server` or when it starts app-server thread/turn
requests. That means NSC behavior can still depend on whichever global Codex
profile/config the local environment happens to apply. For a workflow that is
meant to be deterministic, that is too loose.

# Locked Decisions

- Keep NSC's existing default model as `gpt-5.4`.
- Keep NSC's existing default reasoning effort as `medium`.
- Make fast-tier selection explicit in NSC code rather than depending on user
  config.
- Validate the explicit fast-tier contract with unit tests in the repo.
- Do not broaden this change into other prompt/model-policy changes.

# Scope

In scope:

- Add one explicit fast-tier constant and use it in the NSC app-server adapter.
- Make the `codex app-server` spawn path pass `service_tier=\"fast\"`
  explicitly.
- Update unit tests to assert the explicit fast-tier contract.
- Update user-facing documentation in `README.md` so the default prompt/app
  server contract matches the code.

Out of scope:

- Changing the model from `gpt-5.4`.
- Changing reasoning effort from `medium`.
- Changing any global `~/.codex/config.toml` defaults.
- Broader prompt/workflow changes unrelated to explicit fast-tier selection.

# Implementation Steps With Concrete Files

1. Update `src/app_server.rs`.
   - Add a `DEFAULT_SERVICE_TIER` constant set to `\"fast\"`.
   - Extract the `codex app-server --listen stdio://` launch args into a small
     helper so the explicit spawn contract is easy to assert in tests.
   - Add `-c service_tier=\"fast\"` to that launch arg list so NSC requests
     fast tier before protocol initialization.
2. Extend unit tests in `src/app_server.rs`.
   - Add a helper test around the spawned CLI arg list so it fails if the
     explicit `service_tier=\"fast\"` override disappears.
3. Update `tests/app_server_adapter.rs`.
   - Assert the explicit default service-tier constant alongside model and
    reasoning defaults.
4. Update `README.md`.
   - Expand the prompt/app-server contract text so it states NSC starts Codex
     App Server turns with model `gpt-5.4`, reasoning effort `medium`, and fast
     service tier explicitly requested by NSC.

# Tests

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

# Risks + Mitigations

- Risk: the app-server JSON protocol may not accept an explicit service-tier
  field on thread/turn start.
  - Mitigation: avoid speculative protocol fields and enforce fast-tier
    selection only through the guaranteed CLI spawn override.
- Risk: tests become brittle if they assert large JSON blobs or long arg lists
  too literally.
  - Mitigation: keep assertions focused on the new fast-tier keys/args and the
    existing model/reasoning contract.
- Risk: docs drift from code again.
  - Mitigation: update `README.md` in the same change and keep adapter default
    tests explicit.

# Done Definition

- `src/app_server.rs` explicitly requests `service_tier=\"fast\"` in the NSC
  Codex launch path.
- Adapter tests fail if the explicit fast-tier request is removed.
- `README.md` states the same default contract that the code enforces.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D
  warnings`, and `cargo test` all pass.
