use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use beanz::complexity::{
    baseline_bytes, baseline_complexity, bytes_delta, collect_source_files, compute_deltas,
    cyclomatic, files_delta, watchable_directories, Language,
};

#[test]
fn from_extension_maps_supported_languages() {
    assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    assert_eq!(Language::from_extension("py"), Some(Language::Python));
    assert_eq!(Language::from_extension("java"), Some(Language::Java));
    assert_eq!(Language::from_extension("txt"), None);
}

#[test]
fn cyclomatic_rust_straight_line_returns_one() {
    assert_eq!(cyclomatic("fn a() {}", Language::Rust), 1);
}

#[test]
fn cyclomatic_rust_single_branch_returns_two() {
    let source = "fn a(x: i32) -> i32 { if x > 0 { 1 } else { 2 } }";
    assert_eq!(cyclomatic(source, Language::Rust), 2);
}

#[test]
fn cyclomatic_rust_logical_and_adds_one() {
    let source = "fn a(x: bool, y: bool) { if x && y {} }";
    assert_eq!(cyclomatic(source, Language::Rust), 3);
}

#[test]
fn cyclomatic_rust_match_counts_each_arm() {
    let source = "fn a(x: i32) { match x { 0 => {}, 1 => {}, _ => {} } }";
    assert_eq!(cyclomatic(source, Language::Rust), 4);
}

#[test]
fn cyclomatic_python_single_branch_returns_two() {
    let source = "def a(x):\n    if x:\n        return 1\n    return 2\n";
    assert_eq!(cyclomatic(source, Language::Python), 2);
}

#[test]
fn cyclomatic_python_elif_and_boolean_count() {
    let source = "def a(x, y):\n    if x and y:\n        pass\n    elif x:\n        pass\n";
    assert_eq!(cyclomatic(source, Language::Python), 4);
}

#[test]
fn cyclomatic_java_single_branch_returns_two() {
    let source = "class C { int a(int x){ if (x>0) return 1; return 2; } }";
    assert_eq!(cyclomatic(source, Language::Java), 2);
}

#[test]
fn cyclomatic_java_logical_or_adds_one() {
    let source = "class C { void a(boolean x, boolean y){ if (x || y) {} } }";
    assert_eq!(cyclomatic(source, Language::Java), 3);
}

#[test]
fn collect_source_files_finds_sources_and_skips_ignored() {
    let root = std::env::temp_dir().join(format!(
        "beanz-cx-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    let kept = root.join("src").join("lib.rs");
    fs::write(&kept, "fn a() {}").unwrap();
    fs::write(root.join("src").join("notes.txt"), "ignore").unwrap();
    fs::write(root.join("target").join("gen.rs"), "fn b() {}").unwrap();

    let found = collect_source_files(&root);

    assert_eq!(found, vec![kept]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn collect_source_files_returns_empty_for_missing_root() {
    let missing = PathBuf::from("/nonexistent/beanz/root");
    assert!(collect_source_files(&missing).is_empty());
}

#[test]
fn watchable_directories_excludes_ignored_top_level() {
    let root = std::env::temp_dir().join(format!(
        "beanz-watch-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let kept = root.join("crate-a");
    fs::create_dir_all(&kept).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join("root.rs"), "fn a() {}").unwrap();

    assert_eq!(watchable_directories(&root), vec![kept]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn baseline_complexity_scores_each_source_file() {
    let root = std::env::temp_dir().join(format!(
        "beanz-baseline-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(&root).unwrap();
    let rust = root.join("a.rs");
    let python = root.join("b.py");
    fs::write(&rust, "fn a(x: i32) { if x > 0 {} }").unwrap();
    fs::write(&python, "def a():\n    pass\n").unwrap();

    let baseline = baseline_complexity(&root);

    assert_eq!(baseline.get(&rust), Some(&2));
    assert_eq!(baseline.get(&python), Some(&1));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn compute_deltas_reports_increase_decrease_and_removal() {
    let path_up = PathBuf::from("/repo/up.rs");
    let path_down = PathBuf::from("/repo/down.rs");
    let path_gone = PathBuf::from("/repo/gone.rs");
    let path_same = PathBuf::from("/repo/same.rs");

    let baseline = HashMap::from([
        (path_up.clone(), 3),
        (path_down.clone(), 9),
        (path_gone.clone(), 4),
        (path_same.clone(), 5),
    ]);
    let current = HashMap::from([
        (path_up.clone(), 8),
        (path_down.clone(), 7),
        (path_same.clone(), 5),
    ]);

    let deltas = compute_deltas(&baseline, &current, &HashMap::new(), &HashMap::new());

    let summarized: Vec<(PathBuf, i64)> = deltas
        .iter()
        .map(|delta| (delta.path().to_path_buf(), delta.delta()))
        .collect();
    assert_eq!(
        summarized,
        vec![(path_up, 5), (path_gone, -4), (path_down, -2)]
    );
}

#[test]
fn compute_deltas_nonsource_file_reports_byte_change() {
    let path_notes = PathBuf::from("/repo/notes.txt");

    let baseline_bytes = HashMap::from([(path_notes.clone(), 10)]);
    let current_bytes = HashMap::from([(path_notes.clone(), 210)]);

    let deltas = compute_deltas(
        &HashMap::new(),
        &HashMap::new(),
        &baseline_bytes,
        &current_bytes,
    );

    let summarized: Vec<(PathBuf, i64)> = deltas
        .iter()
        .map(|delta| (delta.path().to_path_buf(), delta.delta()))
        .collect();
    assert_eq!(summarized, vec![(path_notes, 200)]);
}

#[test]
fn disk_scan_trajectory_rises_then_falls() {
    let root = std::env::temp_dir().join(format!(
        "beanz-disk-traj-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("Base.java"), "class Base {}").unwrap();

    let baseline = baseline_bytes(&root);
    let dump = src.join("Dump.java");
    fs::write(
        &dump,
        "class Dump { void a(int x) { if (x > 0) {} if (x < 0) {} } }",
    )
    .unwrap();
    let expanded = baseline_bytes(&root);
    let bytes_up = bytes_delta(&baseline, &expanded);
    let files_up = files_delta(&baseline, &expanded);
    assert!(bytes_up > 0);
    assert_eq!(files_up, 1);

    fs::remove_file(&dump).unwrap();
    let trimmed = baseline_bytes(&root);
    assert_eq!(bytes_delta(&baseline, &trimmed), 0);
    assert_eq!(files_delta(&baseline, &trimmed), 0);

    let cc_baseline = baseline_complexity(&root);
    fs::write(
        &dump,
        "class Dump { void a(int x) { if (x > 0) {} if (x < 0) {} } }",
    )
    .unwrap();
    let cc_expanded = baseline_complexity(&root);
    let cc_up = i64::from(cc_expanded.values().sum::<u32>())
        - i64::from(cc_baseline.values().sum::<u32>());
    assert!(cc_up > 0);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn engine_detects_new_file_after_start() {
    use beanz::complexity::ComplexityEngine;

    let root = std::env::temp_dir().join(format!(
        "beanz-engine-new-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(root.join("src")).unwrap();

    let mut engine = ComplexityEngine::new(root.clone());
    engine.start().unwrap();
    fs::write(
        root.join("src").join("Fresh.java"),
        "class Fresh { void a(int x) { if (x > 0) {} } }",
    )
    .unwrap();
    engine.sync_from_session(&[]);

    assert!(engine.bytes_delta() > 0);
    assert!(engine.introduced() > 0);
    engine.stop();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn engine_detects_markdown_file_as_artifact_bytes_only() {
    use beanz::complexity::ComplexityEngine;

    let root = std::env::temp_dir().join(format!(
        "beanz-engine-artifact-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(&root).unwrap();

    let mut engine = ComplexityEngine::new(root.clone());
    engine.start().unwrap();
    fs::write(root.join("PRD.md"), "# Product Requirements\n\nSome prose.").unwrap();
    engine.sync_from_session(&[]);

    assert!(engine.bytes_delta() > 0);
    assert_eq!(engine.files_delta(), 0);
    assert_eq!(engine.introduced(), 0);
    engine.stop();
    fs::remove_dir_all(&root).ok();
}
