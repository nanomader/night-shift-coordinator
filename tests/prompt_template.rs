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
}
