# Tasks History

- 2026-04-12: Implemented unreadable-slice admission checks, hung-run timeout handling, and repo-relative slice-path prompting from plan `done_docs/_done_nsc_followup_runtime_fixes_plan.md`.
- 2026-04-12: Rewrote the built-in NSC single-slice prompt around the pre-plan checkpoint, `$plan-author-review-loop`, `$implementation-new-branch-pr`, and bounded GitHub Codex review polling from plan `done_docs/_donePLAN_nsc_prompt_night_shift_rewrite_20260412.md`.
- 2026-04-12: Added a separate `debug` command so verbose slice lifecycle output stays opt-in, from plan `done_docs/_done_nsc_debug_command_plan.md`.
- 2026-04-12: Switched slice worktrees to fetched `origin/main` and tightened the built-in merge gate around the explicit Codex no-major-issues review comment from plan `done_docs/_done_nsc_origin_base_and_merge_prompt_plan.md`.
- 2026-04-12: Added explicit `--schedule wave-dirs` execution with ordered child wave-dirs, merged-success unlock verification, and prompt result markers from plan `done_docs/_done_nsc_wave_dirs_implementation_plan.md`.
- 2026-04-12: Hardened execution-truth validation, static batch membership, run-level stop reporting, and frozen-base worktree behavior from plan `done_docs/_done_nsc_execution_truth_hardening_plan.md`.
- 2026-04-13: Set explicit Codex App Server defaults to model `gpt-5.4` with reasoning effort `medium` from plan `done_docs/_done_nsc_gpt54_medium_defaults_plan.md`.
- 2026-04-14: Extended the built-in GitHub Codex review polling window to 20 minutes with 60-second polls and documented the manual-follow-up timeout contract from plan `done_docs/_done_nsc_review_poll_window_plan.md`.
- 2026-04-14: Shortened the built-in GitHub Codex review wait window to 10 minutes and added one same-head retry before manual follow-up from plan `done_docs/_done_nsc_review_retry_plan.md`.
- 2026-04-14: Added prompt-driven `NSC_PROGRESS` phase tracking in debug mode, removed the periodic heartbeat output, and persisted the latest phase in recovery JSON from plan `done_docs/_done_nsc_progress_markers_plan.md`.
