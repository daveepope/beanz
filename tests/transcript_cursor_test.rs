mod harness_factory;

use std::fs;
use std::path::PathBuf;

use beanz::cursor::{
    edit_ops_from_line, parse_line, read_est_chars_from_line, read_est_chars_from_session,
};
use beanz::transcript::Role;
use beanz::EditOp;
use harness_factory::HarnessFactory;

fn workspace_with_src(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "beanz-transcript-{tag}-{}-{:?}",
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
    let factory = HarnessFactory::cursor();
    let session = factory
        .session()
        .user("why not do it differently")
        .tool_call("Write", r#"{"contents":"abcd"}"#)
        .tool_call(
            "StrReplace",
            r#"{"old_string":"x","new_string":"abcdef"}"#,
        )
        .tool_call(
            "Edit",
            r#"{"path":"src/lib.rs","old_string":"a","new_string":"ab"}"#,
        )
        .tool_call("Grep", r#"{"pattern":"x"}"#)
        .tool_call("Shell", r#"{"command":"ls"}"#)
        .tool_call("Bash", r#"{"command":"ls"}"#)
        .tool_call("Read", r#"{"path":"src/lib.rs"}"#)
        .tool_call("Task", r#"{"prompt":"go"}"#);

    let lines = session.lines();
    let user = parse_line(&lines[0]).unwrap();
    assert_eq!(user.role(), Role::User);
    assert!(user.probe_hits >= 2);

    let write = parse_line(&lines[1]).unwrap();
    assert_eq!(write.edit_bytes, 4);

    let replace = parse_line(&lines[2]).unwrap();
    assert_eq!(replace.edit_bytes, 6);

    let edit = parse_line(&lines[3]).unwrap();
    assert_eq!(edit.edit_bytes, 2);

    let read = parse_line(&lines[4]).unwrap();
    assert_eq!(read.read_ops, 1);

    let shell = parse_line(&lines[5]).unwrap();
    assert_eq!(shell.shell_ops, 1);

    let bash = parse_line(&lines[6]).unwrap();
    assert_eq!(bash.shell_ops, 1);

    let read_tool = parse_line(&lines[7]).unwrap();
    assert_eq!(read_tool.read_ops, 1);

    let other = parse_line(&lines[8]).unwrap();
    assert_eq!(other.read_ops, 0);
    assert_eq!(other.shell_ops, 0);
    assert_eq!(other.edit_bytes, 0);

    let string_user =
        parse_line(r#"{"role":"user","message":{"content":"plain question"}}"#).unwrap();
    assert_eq!(string_user.prompt_chars, 14);

    let string_assistant =
        parse_line(r#"{"role":"assistant","message":{"content":"plain reply"}}"#).unwrap();
    assert_eq!(string_assistant.assistant_chars, 11);

    let array_assistant = parse_line(
        r#"{"role":"assistant","message":{"content":[{"type":"text","text":"hello world"}]}}"#,
    )
    .unwrap();
    assert_eq!(array_assistant.assistant_chars, 11);

    assert!(parse_line("not json").is_none());
    assert!(parse_line(r#"{"role":"user"}"#).is_none());

    let sparse = parse_line(
        r#"{"role":"assistant","message":{"content":[{"type":"tool_use","name":"Write"}]}}"#,
    )
    .unwrap();
    assert_eq!(sparse.edit_bytes, 0);

    let non_text = parse_line(
        r#"{"role":"assistant","message":{"content":[{"type":"image","source":"x"}]}}"#,
    )
    .unwrap();
    assert_eq!(non_text.assistant_chars, 0);

    let empty_text = parse_line(
        r#"{"role":"assistant","message":{"content":[{"type":"text"}]}}"#,
    )
    .unwrap();
    assert_eq!(empty_text.assistant_chars, 0);

    let numeric = parse_line(r#"{"role":"user","message":{"content":42}}"#).unwrap();
    assert_eq!(numeric.prompt_chars, 0);
}

#[test]
fn edit_ops_from_line_session_extracts_write_strreplace_and_server_tool() {
    let factory = HarnessFactory::cursor();
    let write_line = factory.write_at_line("src/main.rs", "fn main() {}");
    let replace_line = factory.str_replace_line("src/main.rs", "main", "entry");
    let edit_line = factory.tool_call_line(
        "Edit",
        r#"{"path":"src/a.rs","old_string":"old","new_string":"new"}"#,
    );
    let server_line = r#"{"role":"assistant","message":{"content":[{"type":"server_tool_use","name":"Write","input":{"path":"src/b.rs","contents":"body"}}]}}"#;

    assert_eq!(
        edit_ops_from_line(&write_line),
        vec![EditOp::Write {
            path: PathBuf::from("src/main.rs"),
            contents: "fn main() {}".to_string(),
        }]
    );
    assert_eq!(
        edit_ops_from_line(&replace_line),
        vec![EditOp::StrReplace {
            path: PathBuf::from("src/main.rs"),
            old_string: "main".to_string(),
            new_string: "entry".to_string(),
        }]
    );
    assert_eq!(
        edit_ops_from_line(&edit_line),
        vec![EditOp::StrReplace {
            path: PathBuf::from("src/a.rs"),
            old_string: "old".to_string(),
            new_string: "new".to_string(),
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
    assert!(edit_ops_from_line(r#"{"role":"assistant"}"#).is_empty());
    assert!(edit_ops_from_line(&factory.tool_call_line("Read", r#"{"path":"x"}"#)).is_empty());
    assert!(edit_ops_from_line(r#"{"role":"assistant","message":{"content":"text"}}"#).is_empty());
    assert!(edit_ops_from_line(
        r#"{"role":"assistant","message":{"content":[{"type":"text","text":"hi"}]}}"#
    )
    .is_empty());
    assert!(edit_ops_from_line(
        r#"{"role":"assistant","message":{"content":[{"type":"tool_use","name":"Write"}]}}"#
    )
    .is_empty());
}

#[test]
fn read_est_chars_session_lifecycle_estimates_workspace_reads() {
    let workspace = workspace_with_src("read-est");
    let data = workspace.join("src/data.txt");
    fs::write(&data, "0123456789").unwrap();
    fs::write(workspace.join("big.txt"), "x".repeat(500)).unwrap();
    let transcript = workspace
        .join("agent-transcripts/uuid/uuid.jsonl");
    fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    fs::write(&transcript, "x".repeat(10_000)).unwrap();

    let factory = HarnessFactory::cursor();
    let rel = serde_json::to_string("src/data.txt").unwrap();
    let abs = serde_json::to_string(data.to_string_lossy().as_ref()).unwrap();
    let big = serde_json::to_string("big.txt").unwrap();
    let skip = serde_json::to_string(transcript.to_string_lossy().as_ref()).unwrap();
    let session = factory
        .session()
        .tool_call("Read", &format!(r#"{{"path":{rel}}}"#))
        .tool_call("Read", &format!(r#"{{"path":{abs}}}"#))
        .tool_call("Read", &format!(r#"{{"path":{big},"limit":1}}"#))
        .tool_call("Read", &format!(r#"{{"path":{skip}}}"#))
        .tool_call("Shell", r#"{"command":"ls"}"#);
    let path = session.to_file();

    assert_eq!(read_est_chars_from_session(&path, &workspace).unwrap(), 100);
    assert_eq!(read_est_chars_from_line("bad", &workspace), 0);
    assert_eq!(
        read_est_chars_from_line(r#"{"role":"assistant"}"#, &workspace),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"role":"assistant","message":{"content":"plain"}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"role":"assistant","message":{"content":[{"type":"tool_use","name":"Read"}]}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            r#"{"role":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{}}]}}"#,
            &workspace
        ),
        0
    );
    assert_eq!(
        read_est_chars_from_line(
            &factory.tool_call_line("Read", r#"{"path":"missing.txt"}"#),
            &workspace
        ),
        0
    );

    fs::remove_file(&path).ok();
    teardown_workspace(&workspace);
}
