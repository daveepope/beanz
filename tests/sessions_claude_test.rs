use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use beanz::claude::{
    find_new_session, latest_session_at, latest_session_in, newest_session, scan_sessions,
    session_root, wait_for_new_session_at, wait_for_new_session_in,
};

fn temp_root(tag: &str) -> PathBuf {
    let unique = format!(
        "beanz-sessions-claude-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    );
    let root = std::env::temp_dir().join(unique);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_session(root: &Path, id: &str) -> PathBuf {
    let path = root.join(format!("{id}.jsonl"));
    fs::write(&path, "{}").unwrap();
    path
}

#[test]
fn session_root_maps_workspace_to_claude_layout() {
    let root = session_root(Path::new("/home/user"), Path::new("/home/user/repos/arena"));
    assert_eq!(
        root,
        Path::new("/home/user/.claude/projects/-home-user-repos-arena")
    );
}

#[test]
fn scan_sessions_collects_flat_jsonl_skips_non_jsonl() {
    let root = temp_root("scan");
    let expected = write_session(&root, "alpha");
    fs::write(root.join("notes.txt"), "ignore me").unwrap();
    fs::create_dir_all(root.join("memory")).unwrap();
    fs::write(root.join("memory").join("MEMORY.md"), "ignore me too").unwrap();

    let mut found = scan_sessions(&root);
    found.sort();
    assert_eq!(found, vec![expected]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn find_new_session_returns_path_absent_from_known() {
    let root = temp_root("find-new");
    let known: HashSet<PathBuf> = scan_sessions(&root).into_iter().collect();
    let fresh = write_session(&root, "fresh");

    assert_eq!(find_new_session(&root, &known), Some(fresh));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn find_new_session_returns_none_when_nothing_new() {
    let root = temp_root("find-none");
    write_session(&root, "existing");
    let known: HashSet<PathBuf> = scan_sessions(&root).into_iter().collect();

    assert_eq!(find_new_session(&root, &known), None);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn newest_session_returns_most_recently_modified() {
    let older = PathBuf::from("/sessions/older.jsonl");
    let newer = PathBuf::from("/sessions/newer.jsonl");
    let base = SystemTime::UNIX_EPOCH;
    let dated = vec![
        (base + Duration::from_secs(10), newer.clone()),
        (base + Duration::from_secs(1), older),
    ];

    assert_eq!(newest_session(dated), Some(newer));
}

#[test]
fn latest_session_in_errors_when_no_sessions() {
    let root = temp_root("latest-empty");

    assert!(latest_session_in(&root).is_err());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn latest_session_in_returns_single_existing_session() {
    let root = temp_root("latest-single");
    let only = write_session(&root, "only");

    assert_eq!(latest_session_in(&root).unwrap(), only);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn latest_session_at_returns_seed_from_home_workspace_layout() {
    let home = temp_root("home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();
    let transcripts = session_root(&home, &workspace);
    fs::create_dir_all(&transcripts).unwrap();
    let seed = write_session(&transcripts, "seed");

    assert_eq!(latest_session_at(&home, &workspace).unwrap(), seed);
    fs::remove_dir_all(&home).ok();
}

#[test]
fn wait_for_new_session_at_returns_fresh_after_seed() {
    let home = temp_root("wait-home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();
    let transcripts = session_root(&home, &workspace);
    fs::create_dir_all(&transcripts).unwrap();
    write_session(&transcripts, "seed");

    let transcripts_for_writer = transcripts.clone();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        write_session(&transcripts_for_writer, "fresh")
    });

    let watched = wait_for_new_session_at(&home, &workspace).unwrap();
    let created = writer.join().unwrap();
    assert_eq!(watched, created);
    fs::remove_dir_all(&home).ok();
}

#[test]
fn wait_for_new_session_in_returns_fresh_after_seed() {
    let root = temp_root("wait");
    write_session(&root, "seed");
    let root_for_writer = root.clone();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        write_session(&root_for_writer, "fresh")
    });

    let watched = wait_for_new_session_in(&root).unwrap();
    let created = writer.join().unwrap();
    assert_eq!(watched, created);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn wait_for_new_session_in_missing_root_errors() {
    let root = temp_root("wait-missing");
    fs::remove_dir_all(&root).unwrap();

    assert!(wait_for_new_session_in(&root).is_err());
}
