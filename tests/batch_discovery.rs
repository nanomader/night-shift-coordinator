use std::fs;

use night_shift_coordinator::batch::discover_startup_candidates;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn ignores_non_markdown_and_orders_lexicographically() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("10_last.md"), "c").unwrap();
    fs::write(temp.path().join("01_first.md"), "a").unwrap();
    fs::write(temp.path().join("02_second.md"), "b").unwrap();
    fs::write(temp.path().join("notes.txt"), "ignore").unwrap();

    let candidates = discover_startup_candidates(temp.path()).unwrap();
    let labels = candidates
        .iter()
        .map(|candidate| candidate.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["01_first", "02_second", "10_last"]);
}

#[test]
fn ignores_nsc_directory_contents() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("01_first.md"), "a").unwrap();
    fs::create_dir_all(temp.path().join(".nsc")).unwrap();
    fs::write(temp.path().join(".nsc").join("99_hidden.md"), "hidden").unwrap();

    let candidates = discover_startup_candidates(temp.path()).unwrap();
    let labels = candidates
        .iter()
        .map(|candidate| candidate.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["01_first"]);
}

#[cfg(unix)]
#[test]
fn startup_rejects_unreadable_non_empty_markdown_files() {
    let temp = TempDir::new().unwrap();
    let readable = temp.path().join("01_first.md");
    let unreadable = temp.path().join("02_unreadable.md");
    fs::write(&readable, "a").unwrap();
    fs::write(&unreadable, "hidden").unwrap();
    let original_permissions = fs::metadata(&unreadable).unwrap().permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&unreadable, unreadable_permissions).unwrap();

    let err = discover_startup_candidates(temp.path()).unwrap_err();

    fs::set_permissions(&unreadable, original_permissions).unwrap();
    assert!(err.to_string().contains("path is not readable"));
}
