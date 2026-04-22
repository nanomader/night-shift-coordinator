# Night Shift Coordinator

Night Shift Coordinator (`NSC`) is a local Rust CLI for supervising prepared
Night Shift slice execution.

It is built for one narrow job: run already-prepared Codex work unattended,
from a deterministic fetched `origin/main` base, with one worktree per slice
and operator-visible recovery breadcrumbs when something stops.

**NSC executes prepared Night Shift work. It does not prepare, repair, or
reinterpret that work.**

## When To Use NSC

Use NSC when:

- you already prepared slice markdown files
- you want deterministic execution from fetched `origin/main`
- you want one worktree per running slice
- you want ordered execution only when you ask for it explicitly
- you want clear operator output and repo-local recovery state under `.nsc/`

Do not use NSC when:

- you need slice preparation
- you need dependency inference or DAG scheduling
- you need a general workflow engine
- you want hidden repair, retry, or merge-management behavior
- your prepared inputs do not already exist on the repository state NSC will
  execute from

## Execution Modes

| Mode | Command | Meaning |
| --- | --- | --- |
| Flat batch | `nsc run <slice-folder>` | Run non-empty `*.md` slices from one folder, up to 3 at a time. |
| Ordered wave-dirs | `nsc run <wave-root> --schedule wave-dirs` | Run one child `batchNN_name` directory at a time. |
| Ordered wave-roots | `nsc run <program-root> --schedule wave-roots` | Run one child `waveNN_name` directory at a time, reusing `wave-dirs` rules inside each root. |

Flat mode assumes the slices in one folder are already parallel-safe.
Ordered execution is explicit only.

## Requirements

NSC v1 officially targets:

- macOS
- Linux

You also need:

- `git`
- `codex` with App Server support on your `PATH`
- GitHub access for the built-in prompt workflow, including authenticated PR
  and review capability in the target repository environment
- a target repository that you can run NSC from

## Install

Install from this repository:

```bash
cargo install --path . --locked --force
```

Verify:

```bash
which nsc
nsc --help
```

You can also build without installing:

```bash
cargo build --release
./target/release/nsc --help
```

Homebrew tap packaging is planned, but it is not live from this repository yet.
Until the tap exists, install from source with `cargo install --path . --locked
--force`.

## Quick Start

### Flat Batch

Use this when one folder already contains only parallel-safe slices:

```text
slices/
  01_readme_cleanup.md
  02_cli_examples.md
  03_release_notes.md
```

```bash
nsc run slices/
```

What NSC does:

1. resolves the target repository root
2. force-fetches `origin/main`
3. validates that the discovered slice files exist on that execution base
4. freezes the slice set for the run
5. starts up to 3 slices in parallel

Use debug output when you want lifecycle, phase, and workflow-event lines:

```bash
nsc debug slices/
```

### Ordered Wave-Dirs

Use this when later work must wait for earlier work to land on `main`:

```text
repo_materials/night_shift_batches/validation_entrypoint_wave2_php/
  README.md
  batch02_php_vendor_bin_phpunit/
    01_vendor_bin_phpunit.md
  batch03_composer_test_phpunit/
    01_composer_test_phpunit.md
```

```bash
nsc run repo_materials/night_shift_batches/validation_entrypoint_wave2_php/ --schedule wave-dirs
```

Wave-dir rules:

- each immediate child directory is one wave-dir
- names must match `batchNN_name`
- numbering must be contiguous inside that root
- sibling wave-dirs run one at a time in numeric order
- slices inside one wave-dir still use flat-batch behavior
- NSC fetches `origin/main` before each next wave-dir
- the next wave-dir unlocks only after every slice reports merged success and
  the reported merge SHA verifies on fetched `origin/main`

### Ordered Wave-Roots

Use this when you already have several separately prepared wave roots:

```text
repo_materials/night_shift_batches/validation_entrypoint_program/
  README.md
  wave03_ruby/
    batch01_bundle_exec_rspec/
      01_bundle_exec_rspec.md
    batch02_bin_rspec/
      02_bin_rspec.md
  wave04_ruby_minitest_family/
    batch01_bin_rails_test/
      01_bin_rails_test.md
  wave05_elixir/
    batch01_mix_test/
      01_mix_test.md
```

```bash
nsc run repo_materials/night_shift_batches/validation_entrypoint_program/ --schedule wave-roots
```

Wave-root rules:

- each immediate child directory is one wave root
- names must match `waveNN_name`
- numbering must be contiguous inside that root
- sibling wave roots run one at a time in numeric order
- each wave root reuses existing `wave-dirs` behavior unchanged

## Core Contract

These rules define the shipped behavior:

- the current working directory must be the target repository root
- the supplied target path must resolve inside that repository
- NSC force-fetches `origin/main`, resolves an exact base SHA, and creates
  worktrees from that frozen execution truth
- prepared slice files must already exist on the repository state NSC executes
  from
- once NSC discovers work for a run, treat that discovery as frozen for that
  run
- `NSC_RESULT:` lines are authoritative for ordered unlock behavior
- `NSC_PROGRESS:` and `NSC_EVENT:` lines are informational only
- NSC does not copy or rewrite slice files to compensate for incomplete input

## Slice Input Rules

NSC keeps slice input intentionally light.

- one slice is one markdown file
- flat mode treats every non-empty `*.md` file in the target folder as a slice
  candidate
- unreadable or empty slice files fail fast before launch
- if a discovered slice file is missing from the execution base, NSC stops
  before launch

Minimal slice example:

```md
# 01_auth_cleanup

Implement the auth cleanup described here.

- stay within this slice
- do not widen scope
- stop and report if a missing architecture decision blocks the work
```

## Built-In Prompt Contract

NSC ships with one built-in single-slice prompt in [NSC_PROMPT.md](NSC_PROMPT.md).

By default, NSC:

- uses that built-in prompt
- starts Codex App Server turns with model `gpt-5.4`
- sets reasoning effort to `high`
- passes the assigned slice file path as a repo-relative path
- tells the worker to read that exact slice file before acting
- follows a bounded single-slice workflow with planning, implementation,
  validation, PR creation, explicit Codex review request, review polling, and
  merge gating

That default flow assumes the target repository environment can create pull
requests, post `@codex review` on the PR, inspect GitHub Codex review state,
and merge when the repo workflow allows it.

You can replace the built-in prompt with:

```bash
nsc run <target-root> --prompt-template <path>
nsc debug <target-root> --prompt-template <path>
```

If you use a custom prompt with ordered scheduling, it still needs to satisfy
the result-marker contract below.

If your environment does not provide that GitHub workflow capability, use
`--prompt-template` and supply a prompt that matches the repository's actual
review and merge process.

### Result Markers

Required machine-readable lines for merged success:

```text
NSC_RESULT: MERGED_SUCCESS
NSC_MERGED_MAIN_SHA: <40-hex-sha>
NSC_PR: <number>
```

Common non-success lines:

```text
NSC_RESULT: MANUAL_FOLLOWUP
NSC_RESULT: BLOCKED
NSC_RESULT: STOPPED
```

### Debug Markers

The built-in prompt may emit these progress markers:

```text
NSC_PROGRESS: PLANNING
NSC_PROGRESS: IMPLEMENTING
NSC_PROGRESS: WAITING_FOR_REVIEW
NSC_PROGRESS: FIXING_REVIEW_FINDINGS
NSC_PROGRESS: WAITING_FOR_MERGE
```

NSC also recognizes these workflow-event markers in debug output:

```text
NSC_EVENT: REVIEW_POLLING_STARTED
NSC_EVENT: REVIEW_RETRY_POSTED
NSC_EVENT: FOLLOWUP_PUSHED
NSC_EVENT: MERGE_STARTED
```

`NSC_PROGRESS:` and `NSC_EVENT:` do not affect scheduling semantics.

## Runtime Output

`nsc run ...` keeps output quieter.
`nsc debug ...` adds lifecycle, phase, and workflow-event lines while a slice
is still running.

Typical normal output:

```text
Night Shift Coordinator v0.1.0

Found 5 slices in folder
Starting 3 slices: 01_auth_cleanup, 02_docs_pass, 03_runner_fix

[03_runner_fix] finished normally
  result: merged_success pr=42 merged_main_sha=0123456789abcdef0123456789abcdef01234567
  final_message: done
[02_docs_pass] needs attention
  reason: codex app-server run timed out after 28800 seconds
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/02_docs_pass
```

Typical debug output:

```text
Night Shift Coordinator v0.1.0

Found 1 slices in folder
Starting 1 slices: 01_vendor_bin_phpunit

[01_vendor_bin_phpunit] starting
[01_vendor_bin_phpunit] running
  thread_id: thread-123
  worktree: /repo/.nsc/worktrees/20260412-220501/01_vendor_bin_phpunit
[01_vendor_bin_phpunit] phase: planning
[01_vendor_bin_phpunit] phase: implementing
[01_vendor_bin_phpunit] phase: waiting for review
[01_vendor_bin_phpunit] event: review polling started
```

Operator-facing meanings:

- `finished normally`: the App Server turn completed and NSC captured the final
  prompt-owned terminal message
- `needs attention`: launch, runtime, protocol, or timeout handling did not
  complete normally, including long app-server protocol silence after a slice
  already started
- `Starting wave-dir` / `Wave-dir finished` / `Wave-dir stopped`: ordered
  `wave-dirs` progress
- `Starting wave root` / `Wave root finished` / `Wave root stopped`: ordered
  `wave-roots` progress
- `Fatal startup failure`: NSC rejected the initial target before launching
  anything
- `Run stopped`: a later internal failure stopped the run after startup had
  already succeeded

## Runtime Files And Recovery

NSC creates repo-local runtime files under `.nsc/`:

```text
.nsc/
  worktrees/
    <run_scope>/
      <slice_label>/
  recovery/
    <run_scope>/
      <slice_label>.json
```

Recovery JSON stays intentionally small. It stores:

- `slice_file_path`
- `worktree_path`
- `thread_id`
- `status`
- optional `progress` as the last observed informational progress marker
- optional `progress_updated_at` for the last observed progress marker change
- optional `result`
- optional `pr_number`
- optional `merged_main_sha`
- optional `completed_at` for terminal slice outcomes, including terminal
  `needs attention` states

This is for manual recovery only. It is not a restart system.

For finished slices, terminal truth is the finished status plus any parsed
terminal-result metadata above. `progress` remains informational only and may
still show the last observed phase such as `waiting_for_merge`.
Malformed terminal markers from an otherwise finished prompt-owned turn are
reported as `needs attention` and persist terminal recovery breadcrumbs rather
than leaving stale running state behind. The same applies when a started slice
goes silent at the app-server protocol layer and NSC converts that stall into a
terminal `needs attention` outcome.

When a slice needs attention, NSC surfaces the slice label, reason, worktree
path, and thread id when available. That is enough context to inspect the
worktree and, when appropriate, resume the thread manually:

```bash
codex resume <thread_id>
```

## Failure Model

Startup failures include:

- invalid target directory
- no slice candidates or no ordered children in the chosen mode
- unreadable or empty slice files
- prepared slice files missing from the execution base
- malformed wave-dir or wave-root naming
- numbering gaps in ordered modes

Mid-run attention states include:

- App Server timeout
- App Server protocol failure
- launch failure

Ordered modes stop before the next unit starts if the current unit does not
satisfy the merged-success contract.

## Current Limits

NSC v1 is intentionally narrow:

- max parallel slices is hardcoded to `3`
- no dependency graph
- no overlap scheduler
- no TUI
- no automatic retry
- no dynamic refill after startup
- no supervisor restart or run resume
- no hidden GitHub or merge orchestration beyond the built-in prompt contract

NSC also does not:

- decide whether slices are logically safe together
- infer wave structure from arbitrary folder names
- judge whether an implementation is semantically correct
- replace repo-specific workflow rules

## Development

Useful local commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Packaging

This repository is being prepared for a separate Homebrew tap.

- target install after the tap exists: `brew install nanomader/tap/night-shift-coordinator`
- plain `brew install night-shift-coordinator` is not a current promise
- Homebrew will install the `nsc` binary only
- runtime requirements such as `git`, `codex`, and GitHub review workflow
  access remain explicit operator requirements

## Repo Docs

- [README.md](README.md): product contract and operator-facing behavior
- [AGENTS.md](AGENTS.md): contributor decision policy for everyday NSC work
- [CONTRIBUTING.md](CONTRIBUTING.md): contribution workflow and repo expectations
- [NSC_PROMPT.md](NSC_PROMPT.md): built-in worker prompt contract

## License

Apache License 2.0. See [LICENSE](LICENSE).
