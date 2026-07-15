use std::path::{Path, PathBuf};
use std::process::ExitCode;

use beanz::cli::{
    display_path, format_delta, format_report, parse_args, resolve_session, workspace_for_run,
    ParsedArgs,
};
use beanz::claude::session_root as claude_session_root;
use beanz::cursor::session_root;
use beanz::{run, AgentHarness, ComplexityDelta, DebtTable, Features, Leniency};

fn scored_parsed_args() -> ParsedArgs {
    ParsedArgs {
        command: "score".to_string(),
        harness: "cursor".to_string(),
        path: None,
        workspace: None,
        home: None,
        watch_ticks: None,
        verbose: false,
        lenient: false,
        strict: false,
    }
}

#[test]
fn parse_args_empty_argv_defaults_watch() {
    let parsed = parse_args(&[]).unwrap().unwrap();
    assert_eq!(parsed.command, "watch");
    assert_eq!(parsed.harness, "claude");
    assert!(parsed.path.is_none());
}

#[test]
fn parse_args_score_path_parses_flags() {
    let parsed = parse_args(&[
        "score".to_string(),
        "/tmp/session.jsonl".to_string(),
        "--verbose".to_string(),
        "--strict".to_string(),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(parsed.command, "score");
    assert!(parsed.verbose);
    assert!(parsed.strict);
}

#[test]
fn parse_args_bare_path_defaults_watch() {
    let parsed = parse_args(&["/tmp/session.jsonl".to_string()])
        .unwrap()
        .unwrap();
    assert_eq!(parsed.command, "watch");
    assert_eq!(parsed.path.as_deref(), Some("/tmp/session.jsonl"));
}

#[test]
fn parse_args_extra_positionals_returns_error() {
    assert!(parse_args(&[
        "watch".to_string(),
        "a.jsonl".to_string(),
        "extra".to_string(),
    ])
    .is_err());
}

#[test]
fn parse_args_missing_harness_value_returns_error() {
    assert!(parse_args(&["--harness".to_string()]).is_err());
}

#[test]
fn parse_args_unknown_flag_returns_error() {
    assert!(parse_args(&["--foo".to_string()]).is_err());
}

#[test]
fn parse_args_help_flag_returns_none() {
    assert!(parse_args(&["--help".to_string()]).unwrap().is_none());
    assert!(parse_args(&["-h".to_string()]).unwrap().is_none());
}

#[test]
fn run_help_flag_exits_0() {
    assert_eq!(run(vec!["--help".to_string()]), ExitCode::SUCCESS);
    assert_eq!(run(vec!["-h".to_string()]), ExitCode::SUCCESS);
}

#[test]
fn format_report_verbose_shows_leniency_line() {
    let report = beanz::report(
        Features {
            user_turns: 2,
            assistant_turns: 3,
            prompt_chars: 100,
            bytes_delta: 10,
            ..Features::default()
        },
        Leniency::Lenient,
    );
    let table = DebtTable::new();
    let block = format_report(&report, false, true, Leniency::Lenient, "claude", &table);
    assert!(block.contains("harness: claude"));
    assert!(block.contains("leniency: lenient"));
    assert!(block.contains("leniency=lenient"));
    assert!(block.contains("bytes=10"));
}

#[test]
fn format_report_harness_name_shown_as_own_line() {
    let report = beanz::report(Features::default(), Leniency::Normal);
    let table = DebtTable::new();
    let block = format_report(&report, false, false, Leniency::Normal, "cursor", &table);
    assert!(block.contains("harness: cursor"));
    assert!(block.contains("leniency: normal"));
}

#[test]
fn format_delta_relative_path_shows_cc_change() {
    let cwd = std::env::current_dir().unwrap();
    let line = format_delta(&ComplexityDelta::Complexity {
        path: cwd.join("src/cli.rs"),
        baseline: 1,
        current: 3,
    });
    assert!(line.contains("src/cli.rs"));
    assert!(line.contains("+2"));
}

#[test]
fn format_delta_non_source_path_shows_byte_change() {
    let cwd = std::env::current_dir().unwrap();
    let line = format_delta(&ComplexityDelta::Bytes {
        path: cwd.join("notes.txt"),
        baseline: 0,
        current: 200,
    });
    assert!(line.contains("notes.txt"));
    assert!(line.contains("bytes 0->200"));
    assert!(line.contains("+200"));
}

#[test]
fn display_path_outside_cwd_shows_absolute() {
    let shown = display_path(&PathBuf::from("/var/tmp/beanz-test-path"));
    assert!(shown.contains("beanz-test-path"));
}

#[test]
fn run_unknown_harness_exits_2() {
    assert_eq!(
        run(vec![
            "score".to_string(),
            "--harness".to_string(),
            "windsurf".to_string(),
        ]),
        ExitCode::from(2)
    );
}

#[test]
fn run_conflicting_leniency_flags_exits_2() {
    assert_eq!(
        run(vec![
            "--lenient".to_string(),
            "--strict".to_string(),
            "score".to_string(),
        ]),
        ExitCode::from(2)
    );
}

#[test]
fn parse_args_workspace_and_watch_ticks_parses_flags() {
    let parsed = parse_args(&[
        "watch".to_string(),
        "--workspace".to_string(),
        "/tmp/ws".to_string(),
        "--watch-ticks".to_string(),
        "3".to_string(),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(parsed.workspace.as_deref(), Some("/tmp/ws"));
    assert_eq!(parsed.watch_ticks, Some(3));
}

#[test]
fn workspace_for_run_prefers_workspace_flag() {
    let parsed = parse_args(&[
        "score".to_string(),
        "--workspace".to_string(),
        "/tmp/ws".to_string(),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(workspace_for_run(&parsed), PathBuf::from("/tmp/ws"));
}

#[test]
fn resolve_session_score_empty_transcripts_returns_failure() {
    let home = std::env::temp_dir().join(format!("beanz-cli-home-{}", std::process::id()));
    let workspace = home.join("project");
    std::fs::create_dir_all(session_root(&home, &workspace)).unwrap();
    let parsed = scored_parsed_args();
    assert_eq!(
        resolve_session(AgentHarness::Cursor, &parsed, &workspace, &home),
        Err(ExitCode::FAILURE)
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn resolve_session_score_finds_latest_in_transcripts() {
    let home = std::env::temp_dir().join(format!("beanz-cli-latest-{}", std::process::id()));
    let workspace = home.join("project");
    let transcripts = session_root(&home, &workspace);
    std::fs::create_dir_all(transcripts.join("seed")).unwrap();
    let session = transcripts.join("seed").join("seed.jsonl");
    std::fs::write(&session, "{}").unwrap();
    let parsed = scored_parsed_args();
    assert_eq!(
        resolve_session(AgentHarness::Cursor, &parsed, &workspace, &home).unwrap(),
        session
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn resolve_session_claude_selector_finds_latest_in_claude_transcripts() {
    let home = std::env::temp_dir().join(format!("beanz-cli-claude-latest-{}", std::process::id()));
    let workspace = home.join("project");
    let transcripts = claude_session_root(&home, &workspace);
    std::fs::create_dir_all(&transcripts).unwrap();
    let session = transcripts.join("seed.jsonl");
    std::fs::write(&session, "{}").unwrap();
    let mut parsed = scored_parsed_args();
    parsed.harness = "claude".to_string();
    assert_eq!(
        resolve_session(AgentHarness::Claude, &parsed, &workspace, &home).unwrap(),
        session
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn resolve_session_explicit_path_returns_path() {
    let mut parsed = scored_parsed_args();
    parsed.path = Some("/tmp/explicit.jsonl".to_string());
    let path = resolve_session(
        AgentHarness::Cursor,
        &parsed,
        Path::new("."),
        Path::new("/home"),
    )
    .unwrap();
    assert_eq!(path, PathBuf::from("/tmp/explicit.jsonl"));
}
