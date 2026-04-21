use std::fs;

use night_shift_coordinator::recovery::{RecoveryRecord, recovery_path, write_record};
use night_shift_coordinator::slice::SliceStatus;
use night_shift_coordinator::terminal_result::{ProgressPhase, TerminalResultKind};
use tempfile::TempDir;

#[test]
fn writes_fixed_recovery_json_shape() {
    let temp = TempDir::new().unwrap();
    let record = RecoveryRecord {
        slice_file_path: "slices/01_slice.md".to_string(),
        worktree_path: ".nsc/worktrees/run/01_slice".to_string(),
        thread_id: Some("thread-123".to_string()),
        status: SliceStatus::Running,
        progress: Some(ProgressPhase::Implementing),
        progress_updated_at: Some("2026-04-14T20:00:00Z".to_string()),
        result: None,
        pr_number: None,
        merged_main_sha: None,
        completed_at: None,
    };

    let path = write_record(temp.path(), "run", "01_slice", &record).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(path, recovery_path(temp.path(), "run", "01_slice"));
    assert_eq!(
        value,
        serde_json::json!({
            "slice_file_path": "slices/01_slice.md",
            "worktree_path": ".nsc/worktrees/run/01_slice",
            "thread_id": "thread-123",
            "status": "running",
            "progress": "implementing",
            "progress_updated_at": "2026-04-14T20:00:00Z"
        })
    );
}

#[test]
fn writes_terminal_metadata_when_present() {
    let temp = TempDir::new().unwrap();
    let record = RecoveryRecord {
        slice_file_path: "slices/01_slice.md".to_string(),
        worktree_path: ".nsc/worktrees/run/01_slice".to_string(),
        thread_id: Some("thread-123".to_string()),
        status: SliceStatus::FinishedPromptOwned,
        progress: Some(ProgressPhase::WaitingForMerge),
        progress_updated_at: Some("2026-04-22T00:26:07Z".to_string()),
        result: Some(TerminalResultKind::MergedSuccess),
        pr_number: Some(42),
        merged_main_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        completed_at: Some("2026-04-22T00:26:11Z".to_string()),
    };

    let path = write_record(temp.path(), "run", "01_slice", &record).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(
        value,
        serde_json::json!({
            "slice_file_path": "slices/01_slice.md",
            "worktree_path": ".nsc/worktrees/run/01_slice",
            "thread_id": "thread-123",
            "status": "finished_prompt_owned",
            "progress": "waiting_for_merge",
            "progress_updated_at": "2026-04-22T00:26:07Z",
            "result": "merged_success",
            "pr_number": 42,
            "merged_main_sha": "0123456789abcdef0123456789abcdef01234567",
            "completed_at": "2026-04-22T00:26:11Z"
        })
    );
}
