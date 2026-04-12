use night_shift_coordinator::app_server::TerminalRunOutcome;

#[test]
fn terminal_outcome_debug_representation_is_stable_enough_for_tests() {
    let outcome = TerminalRunOutcome::AttentionNeeded {
        reason: "missing terminal message".to_string(),
    };
    let text = format!("{outcome:?}");
    assert!(text.contains("AttentionNeeded"));
}
