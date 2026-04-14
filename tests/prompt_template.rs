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
        BUILTIN_PROMPT_TEMPLATE.contains("60 seconds")
            && BUILTIN_PROMPT_TEMPLATE.contains("20 minutes"),
        "built-in prompt must define the review polling interval and total window"
    );
    assert!(
        BUILTIN_PROMPT_TEMPLATE.contains("keep polling inside the 20-minute window")
            && BUILTIN_PROMPT_TEMPLATE.contains("only after the full 20-minute window expires"),
        "built-in prompt must wait through the review window before MANUAL_FOLLOWUP"
    );
}
