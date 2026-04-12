## Objective

Update NSC so each slice worktree starts from the latest `origin/main` without
changing the user's main checkout, and update the built-in prompt so a slice is
allowed to merge only after the GitHub Codex review reports no major issues.

## Current Gap / Problem Statement

NSC currently creates worktrees from the local repository `HEAD` in
[src/worktree.rs](src/worktree.rs).
That means a stale local checkout produces stale slice worktrees unless the
operator manually updates the target repo before running NSC.

The built-in prompt in [NSC_PROMPT.md](NSC_PROMPT.md)
also explicitly forbids merge, so a slice can stop at "PR opened and reviewed"
instead of fully landing overnight. That breaks the intended single-slice Night
Shift flow when later slices depend on the earlier slice being landed.

## Locked Decisions

- NSC should fetch from `origin` before launching slices.
- NSC should create slice worktrees from the fetched `origin/main` ref, not the
  current local `HEAD`.
- NSC should not switch, reset, or otherwise mutate the user's main checkout.
- The built-in prompt should allow merge as part of the slice run.
- Merge should happen only after the worker sees a GitHub Codex review comment
  that starts with `Codex Review: Didn't find any major issues.`
- That merge gate should live in the built-in prompt and repo workflow logic,
  not in Rust merge-policy code.
- Debug heartbeat remains at 60 seconds in `debug` mode for now.

## Scope

### In scope

- Add an explicit fetch/base-ref step before worktree creation.
- Change worktree creation to use `origin/main`.
- Fail clearly when `origin/main` cannot be fetched or resolved.
- Update the built-in prompt so merge is allowed and explicitly gated on the
  Codex no-major-issues review comment.
- Update documentation to reflect the new base-ref and merge behavior.
- Add tests for the new Git behavior and prompt wording.

### Out of scope

- Configurable base refs
- Non-`origin` remotes
- Rust-side parsing of GitHub review comments to decide merge readiness
- Auto-merging logic in Rust
- Changing the 60-second debug heartbeat

## Implementation Steps

1. Extend git worktree helpers in
   [src/worktree.rs](src/worktree.rs).
   - Add a helper that runs `git fetch origin main` from the repo root.
   - Add a helper that verifies `origin/main` resolves after fetch.
   - Change `create_worktree` to accept a base ref and use
     `git worktree add --detach <path> <base-ref>`.
   - Use `origin/main` as the supervisor default.

2. Wire the fetched base into supervisor execution in
   [src/supervisor.rs](src/supervisor.rs).
   - Perform the fetch/resolve step once near startup, before any slice launch.
   - Reuse the resolved base ref for all slices in the current NSC run.
   - Keep the user's main checkout untouched.
   - Surface a fatal startup error if the fetch or `origin/main` resolution
     fails.

3. Update error handling in
   [src/error.rs](src/error.rs) if
   needed so fetch/base-ref failures are reported clearly.
   - Reuse existing `Git`-style errors unless a more specific variant is needed
     for operator clarity.

4. Update the built-in prompt in
   [NSC_PROMPT.md](NSC_PROMPT.md).
   - Remove the current `Do not merge as part of this run.` rule.
   - Make the workflow explicitly:
     - open PR
     - request/poll Codex review for the current head
     - merge only after a GitHub Codex review comment begins with
       `Codex Review: Didn't find any major issues.`
   - Make the stop condition explicit when that review state has not yet
     arrived or when findings above LOW remain.

5. Update product docs:
   - [README.md](README.md)
   - [night_shift_coordinator_concept.md](night_shift_coordinator_concept.md)
   - Document that NSC fetches `origin/main` and creates worktrees from that
     remote base.
   - Document that the built-in prompt can merge after the explicit Codex
     no-major-issues review comment.
   - Fix any stale CLI wording in the concept doc while touching it so the docs
     stay aligned with the already-shipped `debug` command.

6. Add or update tests.
   - Add focused git/base-ref tests in
     [src/worktree.rs](src/worktree.rs)
     or a new [tests/worktree.rs](tests/worktree.rs)
     using a temporary repo plus a local bare origin to prove NSC can fetch and
     create from `origin/main` without mutating the main checkout.
   - Add a startup failure test in
     [tests/supervisor_loop.rs](tests/supervisor_loop.rs)
     for missing or unresolvable `origin/main`.
   - Add a built-in prompt-content test in a new
     [tests/prompt_template.rs](tests/prompt_template.rs)
     or an existing appropriate test module asserting the merge gate phrase is
     present.
   - Keep the existing full validation contract green.

## Tests

### Unit

- worktree helper tests for:
  - successful fetch from `origin`
  - successful resolution of `origin/main`
  - worktree creation from a provided base ref
- prompt/built-in-template assertion for the merge gate phrase

### Integration

- temporary git repo with a local bare `origin` in
  [tests/worktree.rs](tests/worktree.rs)
  or equivalent:
  - advance `origin/main` beyond local `HEAD`
  - run the relevant helper path
  - assert the created worktree points at the fetched remote base, not stale
    local `HEAD`
- startup failure path in
  [tests/supervisor_loop.rs](tests/supervisor_loop.rs)
  when `origin/main` is unavailable

### Manual

- in a target repo that is behind `origin/main`, run:
  - `nsc run <slice-folder>`
  - confirm the slice worktree starts from fetched `origin/main`
- inspect the built-in prompt behavior on a slice run and confirm the final
  prompt-owned message can report a merge after the explicit Codex review
  no-major-issues comment

## Risks + Mitigations

- Risk: fetch-based worktree setup adds startup failure modes in repos without a
  usable `origin`.
  - Mitigation: fail fast with a clear Git error before launching any slice.

- Risk: tests around `origin/main` become flaky if they rely on network or the
  real checkout.
  - Mitigation: use local temporary git repos and a local bare remote only.

- Risk: prompt wording is still too vague about when merge is allowed.
  - Mitigation: include the exact review-comment prefix
    `Codex Review: Didn't find any major issues.`

## Done Definition

- NSC fetches `origin/main` before launching slices.
- NSC creates slice worktrees from `origin/main`, not local `HEAD`.
- NSC does not switch or mutate the user's main checkout.
- The built-in prompt explicitly allows merge only after the GitHub Codex
  review comment begins with `Codex Review: Didn't find any major issues.`
- README and concept docs reflect the new base-ref and merge behavior.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test` all pass.
