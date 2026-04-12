use std::fs;

use night_shift_coordinator::recovery::{RecoveryRecord, recovery_path, write_record};
use night_shift_coordinator::slice::SliceStatus;
use tempfile::TempDir;

#[test]
fn writes_fixed_recovery_json_shape() {
    let temp = TempDir::new().unwrap();
    let record = RecoveryRecord {
        slice_file_path: "slices/01_slice.md".to_string(),
        worktree_path: ".nsc/worktrees/run/01_slice".to_string(),
        thread_id: Some("thread-123".to_string()),
        status: SliceStatus::Running,
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
            "status": "running"
        })
    );
}
