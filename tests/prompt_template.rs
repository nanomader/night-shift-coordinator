use night_shift_coordinator::BUILTIN_PROMPT_TEMPLATE;

#[test]
fn builtin_prompt_contains_explicit_merge_gate_phrase() {
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("Codex Review: Didn't find any major issues."),
        "built-in prompt must gate merge on the explicit Codex review phrase"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("repo workflow allows merge")
            && BUILTIN_PROMPT_TEMPLATE.contains("merge")
            && BUILTIN_PROMPT_TEMPLATE.contains("as part of this run"),
        "built-in prompt must allow merge after the review gate is satisfied"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("NSC_RESULT: MERGED_SUCCESS")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_MERGED_MAIN_SHA:")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_PR:"),
        "built-in prompt must emit exact machine-readable markers for wave-dirs mode"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("NSC_PROGRESS: PLANNING")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_PROGRESS: IMPLEMENTING")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_PROGRESS: WAITING_FOR_REVIEW")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_PROGRESS: FIXING_REVIEW_FINDINGS")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_PROGRESS: WAITING_FOR_MERGE"),
        "built-in prompt must emit exact machine-readable progress markers"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("NSC_EVENT: REVIEW_POLLING_STARTED")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_EVENT: FOLLOWUP_PUSHED")
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_EVENT: MERGE_STARTED"),
        "built-in prompt must emit exact machine-readable debug event markers"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE
            .contains("Emit a progress line only when the phase actually changes."),
        "built-in prompt must keep progress markers coarse and non-noisy"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("60 seconds")
            && BUILTIN_PROMPT_TEMPLATE.contains("10 minutes"),
        "built-in prompt must define the review polling interval and total window"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("first review window")
            && BUILTIN_PROMPT_TEMPLATE.contains("scoped to the current PR head")
            && BUILTIN_PROMPT_TEMPLATE.contains("PR lifetime"),
        "built-in prompt must describe the review window as head-scoped, not PR-scoped"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE
            .contains("newly pushed PR head restarts the full head-scoped review")
            && BUILTIN_PROMPT_TEMPLATE.contains("cycle from zero")
            && BUILTIN_PROMPT_TEMPLATE.contains("fresh 10-minute window")
            && BUILTIN_PROMPT_TEMPLATE.contains("carrying over elapsed")
            && BUILTIN_PROMPT_TEMPLATE.contains("wait time from head A"),
        "built-in prompt must restart the review cycle from zero after a follow-up push creates a new head"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("final immediate head-scoped")
            && BUILTIN_PROMPT_TEMPLATE.contains("comment for the current PR head"),
        "built-in prompt must do a final head-scoped re-check before MANUAL_FOLLOWUP"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("not a clean review result")
            && BUILTIN_PROMPT_TEMPLATE.contains("not a")
            && BUILTIN_PROMPT_TEMPLATE.contains("content finding by itself"),
        "built-in prompt must distinguish review timeout from real findings"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("total wait time used for the current PR head")
            && BUILTIN_PROMPT_TEMPLATE.contains("do not fold that")
            && BUILTIN_PROMPT_TEMPLATE.contains("current-head wait total"),
        "built-in prompt must require current-head-scoped wait reporting in the final message"
    );
    assert!(
        !BUILTIN_PROMPT_TEMPLATE.contains("@codex review"),
        "built-in prompt must not require a manual Codex review request"
    );
}

#[test]
fn builtin_prompt_requires_same_family_invariant_sweep_for_launcher_rows() {
    assert!(
        BUILTIN_PROMPT_TEMPLATE
            .contains("if the slice adds, widens, or changes precedence of one admitted")
            && BUILTIN_PROMPT_TEMPLATE.contains("launcher/entrypoint row")
            && BUILTIN_PROMPT_TEMPLATE.contains("especially a newly highest-precedence row")
            && BUILTIN_PROMPT_TEMPLATE.contains("do not")
            && BUILTIN_PROMPT_TEMPLATE.contains("stop at the literal admitted argv"),
        "built-in prompt must require a same-row sweep when adding, widening, or reprioritizing one launcher row"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("executable-bit requirements")
            && BUILTIN_PROMPT_TEMPLATE
                .contains("tracked literal path vs resolved target trackedness")
            && BUILTIN_PROMPT_TEMPLATE.contains("symlink behavior")
            && BUILTIN_PROMPT_TEMPLATE.contains("untracked local state changing admission")
            && BUILTIN_PROMPT_TEMPLATE
                .contains("artifact selector/evidence metadata matching the tracked evidence")
            && BUILTIN_PROMPT_TEMPLATE
                .contains("precedence interactions with already-landed fallback rows")
            && BUILTIN_PROMPT_TEMPLATE
                .contains("can convert a previously runnable\n       repo into a required failure"),
        "built-in prompt must enumerate adjacent invariants for launcher-row changes"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE
            .contains("after fixing a review finding on such an admitted launcher/entrypoint row")
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "do not patch only the literal review comment if sibling cases on the same"
            )
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "row could still violate determinism, debuggability, or the locked support"
            )
            && BUILTIN_PROMPT_TEMPLATE.contains("contract"),
        "built-in prompt must require the same-family sweep during review-fix loops too"
    );
}
