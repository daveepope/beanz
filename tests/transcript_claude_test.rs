mod harness_factory;

use std::fs;
use std::path::PathBuf;

use beanz::claude::{
    edit_ops_from_line, parse_line, read_est_chars_from_line, read_est_chars_from_session,
};
use beanz::transcript::Role;
use beanz::EditOp;
use harness_factory::HarnessFactory;

fn workspace_with_src(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "beanz-transcript-claude-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn teardown_workspace(workspace: &PathBuf) {
    fs::remove_dir_all(workspace).ok();
}

#[test]
fn parse_line_mixed_session_records_roles_tools_and_probes() {
    let factory = HarnessFactory::claude();
    let session = factory
        .session()
        .user("why not do it differently")
        .tool_call("Write", r#"{"content":"abcd"}"#)
        .tool_call(
            "Edit",
            r#"{"file_path":"src/lib.rs","old_string":"x","new_string":"abcdef"}"#,
        )
        .tool_call("Grep", r#"{"pattern":"x"}"#)
        .tool_call("Bash", r#"{"command":"ls"}"#)
        .tool_call("Read", r#"{"file_path":"src/lib.rs"}"#)
        .tool_call("Task", r#"{"prompt":"go"}"#);

    let lines = session.lines();
    let user = parse_line(&lines[0]).unwrap();
    assert_eq!(user.role(), Role::User);
    assert!(user.probe_hits >= 2);

    let write = parse_line(&lines[1]).unwrap();
    assert_eq!(write.code_edit_bytes, 4);

    let edit = parse_line(&lines[2]).unwrap();
    assert_eq!(edit.code_edit_bytes, 6);

    let read_pattern = parse_line(&lines[3]).unwrap();
    assert_eq!(read_pattern.read_ops, 1);

    let bash = parse_line(&lines[4]).unwrap();
    assert_eq!(bash.shell_ops, 1);

    let read_tool = parse_line(&lines[5]).unwrap();
    assert_eq!(read_tool.read_ops, 1);

    let other = parse_line(&lines[6]).unwrap();
    assert_eq!(other.read_ops, 0);
    assert_eq!(other.shell_ops, 0);
    assert_eq!(other.code_edit_bytes, 0);

    let string_user =
        parse_line(r#"{"type":"user","message":{"role":"user","content":"plain question"}}"#)
            .unwrap();
    assert_eq!(string_user.prompt_chars, 14);

    let string_assistant = parse_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":"plain reply"}}"#,
    )
    .unwrap();
    assert_eq!(string_assistant.assistant_chars, 11);

    let array_assistant = parse_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hello world"}]}}"#,
    )
    .unwrap();
    assert_eq!(array_assistant.assistant_chars, 11);

    assert!(parse_line("not json").is_none());
    assert!(parse_line(r#"{"type":"user"}"#).is_none());
    assert!(parse_line(r#"{"type":"mode","mode":"normal"}"#).is_none());

    let sparse = parse_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write"}]}}"#,
    )
    .unwrap();
    assert_eq!(sparse.code_edit_bytes, 0);

    let numeric =
        parse_line(r#"{"type":"user","message":{"role":"user","content":42}}"#).unwrap();
    assert_eq!(numeric.prompt_chars, 0);
}

#[test]
fn parse_line_tool_result_only_returns_none() {
    let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"ok"}]}}"#;
    assert!(parse_line(line).is_none());
}

#[test]
fn parse_line_user_text_with_tool_result_records_prompt_chars() {
    let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"ok"},{"type":"text","text":"why not"}]}}"#;
    let event = parse_line(line).unwrap();
    assert_eq!(event.prompt_chars, 7);
}

#[test]
fn parse_line_multiedit_tool_use_sums_code_edit_bytes() {
    let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"MultiEdit","input":{"file_path":"src/lib.rs","edits":[{"old_string":"a","new_string":"bb"},{"old_string":"c","new_string":"ddd"}]}}]}}"#;
    let event = parse_line(line).unwrap();
    assert_eq!(event.code_edit_bytes, 5);
    assert_eq!(event.artifact_edit_bytes, 0);
}

#[test]
fn parse_line_edit_markdown_path_records_artifact_edit_bytes() {
    let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"docs/PRD.md","old_string":"a","new_string":"abcd"}}]}}"#;
    let event = parse_line(line).unwrap();
    assert_eq!(event.artifact_edit_bytes, 4);
    assert_eq!(event.code_edit_bytes, 0);
}

#[test]
fn edit_ops_from_line_multiedit_extracts_multiple_str_replace() {
    let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"MultiEdit","input":{"file_path":"src/lib.rs","edits":[{"old_string":"a","new_string":"b"},{"old_string":"c","new_string":"d"}]}}]}}"#;
    let ops = edit_ops_from_line(line);
    assert_eq!(
        ops,
        vec![
            EditOp::StrReplace {
                path: PathBuf::from("src/lib.rs"),
                old_string: "a".to_string(),
                new_string: "b".to_string(),
            },
            EditOp::StrReplace {
                path: PathBuf::from("src/lib.rs"),
                old_string: "c".to_string(),
                new_string: "d".to_string(),
            },
        ]
    );
}

#[test]
fn parse_line_sidechain_entry_returns_none() {
    let line = r#"{"type":"user","isSidechain":true,"message":{"role":"user","content":"subagent question"}}"#;
    assert!(parse_line(line).is_none());
}

#[test]
fn edit_ops_from_line_session_extracts_write_edit_and_server_tool() {
    let factory = HarnessFactory::claude();
    let write_line = factory.write_at_line("src/main.rs", "fn main() {}");
    let edit_line = factory.str_replace_line("src/main.rs", "main", "entry");
    let server_line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"server_tool_use","name":"Write","input":{"file_path":"src/b.rs","content":"body"}}]}}"#;

    assert_eq!(
        edit_ops_from_line(&write_line),
        vec![EditOp::Write {
            path: PathBuf::from("src/main.rs"),
            contents: "fn main() {}".to_string(),
        }]
    );
    assert_eq!(
        edit_ops_from_line(&edit_line),
        vec![EditOp::StrReplace {
            path: PathBuf::from("src/main.rs"),
            old_string: "main".to_string(),
            new_string: "entry".to_string(),
        }]
    );
    assert_eq!(
        edit_ops_from_line(server_line),
        vec![EditOp::Write {
            path: PathBuf::from("src/b.rs"),
            contents: "body".to_string(),
        }]
    );

    assert!(edit_ops_from_line("not json").is_empty());
    assert!(edit_ops_from_line(r#"{"type":"assistant"}"#).is_empty());
    assert!(
        edit_ops_from_line(&factory.tool_call_line("Read", r#"{"file_path":"x"}"#)).is_empty()
    );
    assert!(edit_ops_from_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":"text"}}"#
    )
    .is_empty());
    assert!(edit_ops_from_line(
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write"}]}}"#
    )
    .is_empty());
}

#[test]
fn edit_ops_from_line_sidechain_write_returns_empty() {
    let line = r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","content":[{"type":"tool_use","name":"Write","input":{"file_path":"src/sub.rs","content":"body"}}]}}"#;
    assert!(edit_ops_from_line(line).is_empty());
}

#[test]
fn read_est_chars_session_lifecycle_estimates_workspace_reads() {
    let workspace = workspace_with_src("read-est");
    let data = workspace.join("src/data.txt");
    fs::write(&data, "0123456789").unwrap();
    fs::write(workspace.join("big.txt"), "x".repeat(500)).unwrap();
    let transcript = workspace.join(".claude/projects/slug/uuid.jsonl");
    fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    fs::write(&transcript, "x".repeat(10_000)).unwrap();

    let factory = HarnessFactory::claude();
    let rel = serde_json::to_string("src/data.txt").unwrap();
    let abs = serde_json::to_string(data.to_string_lossy().as_ref()).unwrap();
    let big = serde_json::to_string("big.txt").unwrap();
    let skip = serde_json::to_string(transcript.to_string_lossy().as_ref()).unwrap();
    let session = factory
        .session()
        .tool_call("Read", &format!(r#"{{"file_path":{rel}}}"#))
        .tool_call("Read", &format!(r#"{{"file_path":{abs}}}"#))
        .tool_call("Read", &format!(r#"{{"file_path":{big},"limit":1}}"#))
        .tool_call("Read", &format!(r#"{{"file_path":{skip}}}"#))
        .tool_call("Bash", r#"{"command":"ls"}"#);
    let path = session.to_file();

    assert_eq!(read_est_chars_from_session(&path, &workspace).unwrap(), 100);
    assert_eq!(read_est_chars_from_line("bad", &workspace), 0);
    assert_eq!(
        read_est_chars_from_line(r#"{"type":"assistant"}"#, &workspace),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"type":"assistant","message":{"role":"assistant","content":"plain"}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read"}]}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{}}]}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            &factory.tool_call_line("Read", r#"{"file_path":"missing.txt"}"#),
            &workspace
        ),
        0
    );
    let sidechain_read = format!(
        r#"{{"type":"assistant","isSidechain":true,"message":{{"role":"assistant","content":[{{"type":"tool_use","name":"Read","input":{{"file_path":{rel}}}}}]}}}}"#
    );
    assert_eq!(read_est_chars_from_line(&sidechain_read, &workspace), 0);

    fs::remove_file(&path).ok();
    teardown_workspace(&workspace);
}
