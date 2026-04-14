use night_shift_coordinator::terminal_result::{
    ProgressPhase, TerminalResultKind, parse_progress_marker, parse_terminal_result_markers,
};

#[test]
fn parses_merged_success_markers() {
    let markers = parse_terminal_result_markers(
        "done\nNSC_RESULT: MERGED_SUCCESS\nNSC_MERGED_MAIN_SHA: 0123456789abcdef0123456789abcdef01234567\nNSC_PR: 42\n",
    )
    .unwrap()
    .unwrap();

    assert_eq!(markers.result, TerminalResultKind::MergedSuccess);
    assert_eq!(
        markers.merged_main_sha.as_deref(),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(markers.pr_number, Some(42));
}

#[test]
fn parses_non_success_marker_without_sha() {
    let markers = parse_terminal_result_markers("NSC_RESULT: MANUAL_FOLLOWUP\n")
        .unwrap()
        .unwrap();

    assert_eq!(markers.result, TerminalResultKind::ManualFollowup);
    assert_eq!(markers.merged_main_sha, None);
    assert_eq!(markers.pr_number, None);
}

#[test]
fn returns_none_when_markers_are_absent() {
    let markers = parse_terminal_result_markers("plain final message").unwrap();

    assert_eq!(markers, None);
}

#[test]
fn rejects_malformed_sha() {
    let err =
        parse_terminal_result_markers("NSC_RESULT: MERGED_SUCCESS\nNSC_MERGED_MAIN_SHA: nope\n")
            .unwrap_err();

    assert!(err.to_string().contains("invalid NSC_MERGED_MAIN_SHA"));
}

#[test]
fn rejects_malformed_pr() {
    let err =
        parse_terminal_result_markers("NSC_RESULT: MERGED_SUCCESS\nNSC_PR: nope\n").unwrap_err();

    assert!(err.to_string().contains("invalid NSC_PR"));
}

#[test]
fn parses_last_recognized_progress_marker_from_message() {
    let progress =
        parse_progress_marker("note\nNSC_PROGRESS: PLANNING\ntext\nNSC_PROGRESS: IMPLEMENTING\n");

    assert_eq!(progress, Some(ProgressPhase::Implementing));
}

#[test]
fn ignores_unknown_progress_values() {
    let progress = parse_progress_marker("NSC_PROGRESS: SOMETHING_ELSE\n");

    assert_eq!(progress, None);
}
