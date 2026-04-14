use std::fs;

use night_shift_coordinator::wave_roots::discover_wave_roots;
use tempfile::TempDir;

#[test]
fn discovers_goalcoder_style_wave_roots_in_order() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("wave03_ruby")).unwrap();
    fs::create_dir_all(temp.path().join("wave04_ruby_minitest_family")).unwrap();
    fs::write(temp.path().join("README.md"), "context").unwrap();
    fs::write(temp.path().join(".DS_Store"), "").unwrap();

    let wave_roots = discover_wave_roots(temp.path()).unwrap();
    let names = wave_roots
        .iter()
        .map(|wave_root| wave_root.name.as_str())
        .collect::<Vec<_>>();
    let wave_ids = wave_roots
        .iter()
        .map(|wave_root| wave_root.wave_id)
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["wave03_ruby", "wave04_ruby_minitest_family"]);
    assert_eq!(wave_ids, vec![3, 4]);
}

#[test]
fn rejects_invalid_wave_root_name() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("03_ruby")).unwrap();

    let err = discover_wave_roots(temp.path()).unwrap_err();

    assert!(err.to_string().contains("invalid wave root name"));
}

#[test]
fn rejects_gap_in_wave_root_sequence() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("wave03_ruby")).unwrap();
    fs::create_dir_all(temp.path().join("wave05_elixir")).unwrap();

    let err = discover_wave_roots(temp.path()).unwrap_err();

    assert!(err.to_string().contains("not contiguous"));
}

#[test]
fn rejects_unexpected_program_root_entries() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("wave03_ruby")).unwrap();
    fs::write(temp.path().join("notes.txt"), "unexpected").unwrap();

    let err = discover_wave_roots(temp.path()).unwrap_err();

    assert!(err.to_string().contains("unexpected entry in program root"));
}

#[test]
fn rejects_visible_child_directory_with_wrong_grammar() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("wave03_ruby")).unwrap();
    fs::create_dir_all(temp.path().join("batch04_wrong_level")).unwrap();

    let err = discover_wave_roots(temp.path()).unwrap_err();

    assert!(err.to_string().contains("invalid wave root name"));
}

#[test]
fn fails_when_no_wave_roots_are_found() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("README.md"), "only docs").unwrap();

    let err = discover_wave_roots(temp.path()).unwrap_err();

    assert!(err.to_string().contains("no wave roots found"));
}
