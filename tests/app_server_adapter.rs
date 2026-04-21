use night_shift_coordinator::app_server::{
    DEFAULT_MODEL, DEFAULT_REASONING_EFFORT, TerminalRunOutcome,
};

#[test]
fn terminal_outcome_debug_representation_is_stable_enough_for_tests() {
    let outcome = TerminalRunOutcome::AttentionNeeded {
        reason: "missing terminal message".to_string(),
    };
    let text = format!("{outcome:?}");
    assert!(text.contains("AttentionNeeded"));
}

#[test]
fn app_server_defaults_are_explicit() {
    assert_eq!(DEFAULT_MODEL, "gpt-5.4");
    assert_eq!(DEFAULT_REASONING_EFFORT, "high");
}
