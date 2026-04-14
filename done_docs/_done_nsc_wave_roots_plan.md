# Objective

Add an explicit `--schedule wave-roots` mode so NSC can run several prepared
wave roots one after another from one program root, while reusing the existing
`wave-dirs` behavior unchanged inside each wave root.

# Current Gap / Problem Statement

NSC currently supports:

- flat batch mode for one folder of parallel-safe slice files
- `--schedule wave-dirs` for one wave root containing ordered child
  `batchNN_*` directories

That solves one prepared wave root well, but it does not solve the next GoalCoder
layer up: a parent program root that contains several separately prepared wave
roots that must run in deterministic order without manual relaunches.

The current scheduler and output naming also collapse concepts that will become
ambiguous once a higher-level scheduler exists. In particular, some existing
`WaveRoot*` output names actually refer to one `wave-dirs` root rather than an
actual wave root. That terminology needs to be normalized during the change so
the code, logs, and docs clearly distinguish:

- program root
- wave root
- wave-dir

# Locked Decisions

- Add explicit `--schedule wave-roots`.
- Keep default flat mode unchanged.
- Keep `--schedule wave-dirs` behavior unchanged inside one wave root.
- Program-root children must match strict `waveNN_name`.
- Wave-root children remain strict `batchNN_name`.
- No optional letters, suffix ordering, mixed grammar, or fallback parsing.
- Program-root numbering must be contiguous within that root.
- `README.md` and hidden entries are allowed in the program root.
- Any other visible program-root non-directory entry fails fast.
- Any visible child directory in the program root that does not match
  `waveNN_name` fails fast.
- Program-root contents are discovered once at startup and frozen for the whole
  run.
- Wave-root success reuses the existing `wave-dirs` merged-success contract
  exactly.
- No new `NSC_RESULT:` marker family.
- Fetch behavior remains compositional:
  - fetch between wave roots
  - still fetch between child wave-dirs inside each wave root
- Implementation must compose existing runners instead of building a new
  scheduler from scratch.
- Internal/output naming must distinguish:
  - program root
  - wave root
  - wave-dir

# Scope

In:

- CLI support for `--schedule wave-roots`
- program-root discovery and validation
- `src/lib.rs` export wiring for any new discovery module
- supervisor composition for:
  - flat batch runner
  - wave-root runner over flat batch runner
  - program-root runner over wave-root runner
- output/event naming cleanup so logs clearly represent program roots, wave
  roots, and wave-dirs
- documentation updates in `README.md` and `night_shift_coordinator_concept.md`
- deterministic tests for discovery, CLI parsing, and ordered execution
- archived plan and task history update as part of implementation workflow

Out:

- changing default flat-batch semantics
- changing the `wave-dirs` success contract
- dependency inference or DAG scheduling
- dynamic refill in `wave-roots` mode
- recursive arbitrary directory traversal beyond the explicit 3-layer model
- any new result marker family

# Implementation Steps With Concrete Files

1. Extend CLI scheduling in `src/cli.rs` and `tests/cli.rs`.
   - Add `WaveRoots` to `ScheduleMode`.
   - Add parse coverage for:
     - `nsc run <program-root> --schedule wave-roots`
     - `nsc debug <program-root> --schedule wave-roots`

2. Add program-root discovery in a new module, likely `src/wave_roots.rs`, and
   export it from `src/lib.rs`.
   - Implement strict discovery for immediate child directories matching
     `waveNN_name`.
   - Reuse the same structural rules as `wave-dirs` where appropriate:
     - allow `README.md`
     - allow hidden entries
     - fail on any other visible non-directory entry
     - fail on any visible child directory not matching the grammar
     - require contiguous numbering
   - Add focused tests for:
     - valid discovery
     - invalid directory names
     - numbering gaps
     - disallowed visible root entries

3. Normalize scheduler terminology in output and supervisor code.
   - Rename the current `WaveRoot*` output events in `src/output.rs` to
     wave-dir-specific names so they no longer collide with actual wave roots.
   - Add new output events for program-root / wave-root execution, for example:
     - program root summary
     - wave root starting
     - wave root finished
     - wave root stopped
   - Update output rendering and output tests to keep the terminal language
     aligned with the 3-layer model.

4. Refactor the supervisor composition in `src/supervisor.rs`.
   - Keep the existing flat batch runner as the lowest layer.
   - Extract the current `wave-dirs` logic into an explicit wave-root runner
     for one wave root that:
     - discovers `batchNN_*` wave-dirs
     - runs them one at a time
     - reuses flat batch execution inside each wave-dir
     - returns a summary that already enforces the existing merged-success
       contract
   - Add a new program-root runner that:
     - discovers `waveNN_*` wave roots once at startup
     - runs one wave root at a time
     - invokes the wave-root runner fresh for each wave root instead of
       threading a cached base across program-root boundaries
     - stops on the first non-success wave root
     - therefore preserves both fetch layers:
       - a fresh `prepare_default_base_ref()` call at the start of each wave
         root
       - the existing per-wave-dir fetch behavior inside that wave root

5. Reuse existing merged-success verification without inventing new result
   semantics.
   - Keep the current wave-dir unlock checks in `src/supervisor.rs` and
     `src/worktree.rs`.
   - Add one composed wave-root success check that succeeds only when every
     child wave-dir already succeeded under that existing contract.
   - Avoid introducing any new terminal marker parsing.

6. Extend error types in `src/error.rs`.
   - Add explicit errors for invalid program-root entries, invalid wave-root
     names, duplicates, and numbering gaps.
   - Keep messages precise so startup failures point to the exact structural
     violation.

7. Add scheduler tests.
   - Add focused discovery tests for `src/wave_roots.rs`, likely in a new
     `tests/wave_roots.rs`.
   - Add end-to-end supervisor tests, likely in a new
     `tests/wave_root_scheduler.rs`, covering:
     - ordered wave-root execution
     - stop-on-first-non-success wave root
     - reuse of current `wave-dirs` semantics inside each wave root
     - parallel-safe slices inside a later wave-dir of a wave root
   - Update existing tests that reference the old `WaveRoot*` output names and
     cover the new program-root output lines.

8. Update docs.
   - `README.md`:
     - describe the 3-layer scheduling model
     - add `wave-roots` command examples
     - explain strict `waveNN_name` grammar
     - explain that `wave-roots` composes `wave-dirs`
   - `night_shift_coordinator_concept.md`:
     - add the program-root layer and its frozen discovery contract
     - clarify terminology between program root, wave root, and wave-dir

9. Finish workflow artifacts.
   - Move this plan into `done_docs/` with the `_done_` prefix after the code is
     implemented.
   - Append one sentence to `TASKS_HISTORY.md` describing the wave-roots
     scheduler addition and plan filename.

# Tests

Focused tests:

- `cargo test --test cli`
- `cargo test --test wave_dirs`
- `cargo test --test wave_roots`
- `cargo test --test wave_dir_scheduler`
- `cargo test --test wave_root_scheduler`

Full validation contract:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo run -- --help`
- `cargo run -- run --help`
- `cargo run -- debug --help`

Manual verification:

- run `nsc run <program-root> --schedule wave-roots` against a prepared temp
  repo layout and confirm:
  - program roots start in numeric order
  - later wave roots do not start after a non-success wave root
  - child wave-dirs inside a wave root still behave as they do today

# Risks + Mitigations

- Risk: terminology drift leaves the new 3-layer model hard to reason about.
  - Mitigation: rename output and internal types during implementation instead
    of layering new meaning onto old names.

- Risk: the new runner duplicates `wave-dirs` logic instead of composing it.
  - Mitigation: refactor around one flat batch runner, one wave-root runner
    over flat batches, and one program-root runner over wave roots.

- Risk: strict grammar changes could silently diverge from docs.
  - Mitigation: keep discovery rules in dedicated modules with focused tests for
    allowed and disallowed names and entries.

- Risk: fetch semantics become unclear across layers.
  - Mitigation: document explicitly that `wave-roots` adds ordering across wave
    roots while preserving existing per-wave-dir fetch behavior inside each wave
    root.

# Done Definition

The change is done when all of the following are true:

- `--schedule wave-roots` is accepted by the CLI for both `run` and `debug`
- NSC can discover and validate strict `waveNN_name` children from a program
  root
- program-root discovery is frozen at startup
- NSC runs one wave root at a time in contiguous numeric order
- each wave root reuses existing `wave-dirs` behavior unchanged
- the next wave root starts only after the current wave root fully satisfies the
  existing merged-success contract
- no new result marker family was added
- internal and output naming clearly distinguish program root, wave root, and
  wave-dir
- docs explain the 3-layer model accurately
- full local validation passes
- the plan is archived, task history is updated, the branch is pushed, and a PR
  is opened with the required Codex review comment
