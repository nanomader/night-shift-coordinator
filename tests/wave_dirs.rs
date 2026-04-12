use std::fs;

use night_shift_coordinator::wave_dirs::discover_wave_dirs;
use tempfile::TempDir;

#[test]
fn discovers_goalcoder_style_wave_dirs_in_order() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("batch02_php_vendor_bin_phpunit")).unwrap();
    fs::create_dir_all(temp.path().join("batch03_composer_test_phpunit")).unwrap();
    fs::write(temp.path().join("README.md"), "context").unwrap();
    fs::write(temp.path().join(".DS_Store"), "").unwrap();

    let wave_dirs = discover_wave_dirs(temp.path()).unwrap();
    let names = wave_dirs
        .iter()
        .map(|wave_dir| wave_dir.name.as_str())
        .collect::<Vec<_>>();
    let batch_ids = wave_dirs
        .iter()
        .map(|wave_dir| wave_dir.batch_id)
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "batch02_php_vendor_bin_phpunit",
            "batch03_composer_test_phpunit"
        ]
    );
    assert_eq!(batch_ids, vec![2, 3]);
}

#[test]
fn rejects_invalid_wave_dir_name() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("02_php_vendor_bin_phpunit")).unwrap();

    let err = discover_wave_dirs(temp.path()).unwrap_err();

    assert!(err.to_string().contains("invalid wave directory name"));
}

#[test]
fn rejects_gap_in_wave_dir_sequence() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("batch02_first")).unwrap();
    fs::create_dir_all(temp.path().join("batch04_third")).unwrap();

    let err = discover_wave_dirs(temp.path()).unwrap_err();

    assert!(err.to_string().contains("not contiguous"));
}

#[test]
fn rejects_unexpected_non_directory_entries() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("batch02_first")).unwrap();
    fs::write(temp.path().join("notes.txt"), "unexpected").unwrap();

    let err = discover_wave_dirs(temp.path()).unwrap_err();

    assert!(err.to_string().contains("unexpected entry in wave root"));
}

#[test]
fn fails_when_no_wave_dirs_are_found() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("README.md"), "only docs").unwrap();

    let err = discover_wave_dirs(temp.path()).unwrap_err();

    assert!(err.to_string().contains("no wave directories found"));
}
