use std::fs;
use std::path::PathBuf;

use beanz::claude::session_root as claude_session_root;
use beanz::cursor::session_root;
use beanz::{AgentHarness, UnsupportedHarness, Leniency};

#[test]
fn parse_cursor_returns_harness() {
    assert_eq!(AgentHarness::parse("cursor"), Ok(AgentHarness::Cursor));
}

#[test]
fn parse_claude_returns_harness() {
    assert_eq!(AgentHarness::parse("claude"), Ok(AgentHarness::Claude));
}

#[test]
fn parse_mixed_case_and_spaces_returns_harness() {
    assert_eq!(AgentHarness::parse("  Cursor "), Ok(AgentHarness::Cursor));
    assert_eq!(AgentHarness::parse("  Claude "), Ok(AgentHarness::Claude));
}

#[test]
fn parse_unknown_returns_unsupported() {
    let error = AgentHarness::parse("windsurf").unwrap_err();
    assert_eq!(error, UnsupportedHarness("windsurf".to_string()));
}

#[test]
fn name_round_trips_through_parse() {
    for harness in [AgentHarness::Cursor, AgentHarness::Claude] {
        assert_eq!(AgentHarness::parse(harness.name()), Ok(harness));
    }
}

fn temp_home(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "beanz-harness-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn latest_session_at_empty_transcripts_errors() {
    let home = temp_home("latest-empty");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(session_root(&home, &workspace)).unwrap();

    assert!(AgentHarness::Cursor
        .latest_session_at(&home, &workspace)
        .is_err());

    fs::remove_dir_all(&home).ok();
}

#[test]
fn latest_session_at_claude_empty_transcripts_errors() {
    let home = temp_home("latest-empty-claude");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(claude_session_root(&home, &workspace)).unwrap();

    assert!(AgentHarness::Claude
        .latest_session_at(&home, &workspace)
        .is_err());

    fs::remove_dir_all(&home).ok();
}

#[test]
fn wait_for_new_session_at_missing_transcripts_errors() {
    let home = temp_home("wait-at-missing");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();

    assert!(AgentHarness::Cursor
        .wait_for_new_session_at(&home, &workspace)
        .is_err());

    fs::remove_dir_all(&home).ok();
}

#[test]
fn wait_for_new_session_at_claude_missing_transcripts_errors() {
    let home = temp_home("wait-at-missing-claude");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();

    assert!(AgentHarness::Claude
        .wait_for_new_session_at(&home, &workspace)
        .is_err());

    fs::remove_dir_all(&home).ok();
}

#[test]
fn latest_session_env_wrapper_runs() {
    if std::env::var_os("HOME").is_none() {
        return;
    }
    let _ = AgentHarness::Cursor.latest_session();
    let _ = AgentHarness::Claude.latest_session();
}

#[test]
fn open_starts_cursor_harness_for_session_file() {
    let home = temp_home("open");
    let session = home.join("session.jsonl");
    fs::write(&session, "{}\n").unwrap();

    let mut harness = AgentHarness::Cursor.open(session.clone(), Leniency::Normal);
    assert!(harness.start().is_ok());
    harness.stop();

    fs::remove_dir_all(&home).ok();
}

#[test]
fn open_starts_claude_harness_for_session_file() {
    let home = temp_home("open-claude");
    let session = home.join("session.jsonl");
    fs::write(&session, "{}\n").unwrap();

    let mut harness = AgentHarness::Claude.open(session.clone(), Leniency::Normal);
    assert!(harness.start().is_ok());
    harness.stop();

    fs::remove_dir_all(&home).ok();
}
