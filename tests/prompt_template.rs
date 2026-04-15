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
            && BUILTIN_PROMPT_TEMPLATE.contains("NSC_EVENT: REVIEW_RETRY_POSTED")
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
            && BUILTIN_PROMPT_TEMPLATE.contains("one second\n     10-minute window"),
        "built-in prompt must describe the first and second review windows"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains(
            "post exactly one second `@codex review` request on the same unchanged PR head"
        ) && BUILTIN_PROMPT_TEMPLATE.contains("only after both 10-minute windows expire"),
        "built-in prompt must retry once on the same head before MANUAL_FOLLOWUP"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE
            .contains("not a clean review result and not a content finding by itself"),
        "built-in prompt must distinguish review timeout from real findings"
    );
}

#[test]
fn builtin_prompt_requires_same_family_invariant_sweep_for_launcher_rows() {
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains(
            "if the slice adds, widens, or changes precedence of one admitted"
        ) && BUILTIN_PROMPT_TEMPLATE.contains("launcher/entrypoint row")
            && BUILTIN_PROMPT_TEMPLATE.contains("especially a newly highest-precedence row")
            && BUILTIN_PROMPT_TEMPLATE.contains("do not")
            && BUILTIN_PROMPT_TEMPLATE.contains("stop at the literal admitted argv"),
        "built-in prompt must require a same-row sweep when adding, widening, or reprioritizing one launcher row"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("executable-bit requirements")
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "tracked literal path vs resolved target trackedness"
            )
            && BUILTIN_PROMPT_TEMPLATE.contains("symlink behavior")
            && BUILTIN_PROMPT_TEMPLATE.contains("untracked local state changing admission")
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "artifact selector/evidence metadata matching the tracked evidence"
            )
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "precedence interactions with already-landed fallback rows"
            )
            && BUILTIN_PROMPT_TEMPLATE.contains(
                "can convert a previously runnable\n       repo into a required failure"
            ),
        "built-in prompt must enumerate adjacent invariants for launcher-row changes"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains(
            "after fixing a review finding on such an admitted launcher/entrypoint row"
        ) && BUILTIN_PROMPT_TEMPLATE.contains(
            "do not patch only the literal review comment if sibling cases on the same"
        ) && BUILTIN_PROMPT_TEMPLATE.contains(
            "row could still violate determinism, debuggability, or the locked support"
        ) && BUILTIN_PROMPT_TEMPLATE.contains("contract"),
        "built-in prompt must require the same-family sweep during review-fix loops too"
    );
}
