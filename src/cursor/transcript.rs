use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::edits::EditOp;
use crate::transcript::{record_assistant_text, record_tool, record_user_text, Event, ToolKind};

const EDIT_TOOLS: [&str; 3] = ["Write", "StrReplace", "Edit"];
const READ_TOOLS: [&str; 3] = ["Read", "Grep", "Glob"];
const SHELL_TOOLS: [&str; 2] = ["Shell", "Bash"];
const DEFAULT_READ_LINE_CAP: usize = 50;
const AVG_LINE_CHARS: usize = 80;

pub fn edit_ops_from_line(line: &str) -> Vec<EditOp> {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let Some(content) = value.get("message").and_then(|message| message.get("content")) else {
        return Vec::new();
    };
    let mut ops = Vec::new();
    absorb_edit_ops(&mut ops, content);
    ops
}

fn absorb_edit_ops(ops: &mut Vec<EditOp>, content: &Value) {
    match content {
        Value::Array(blocks) => {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("tool_use") | Some("server_tool_use") => {
                        let name = block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        if let Some(input) = block.get("input") {
                            if let Some(op) = edit_op_from_input(name, input) {
                                ops.push(op);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn edit_op_from_input(name: &str, input: &Value) -> Option<EditOp> {
    let path = PathBuf::from(input.get("path").and_then(Value::as_str)?);
    match name {
        "Write" => {
            let contents = input.get("contents").and_then(Value::as_str)?.to_string();
            Some(EditOp::Write { path, contents })
        }
        "StrReplace" | "Edit" => {
            let old_string = input.get("old_string").and_then(Value::as_str)?.to_string();
            let new_string = input.get("new_string").and_then(Value::as_str)?.to_string();
            Some(EditOp::StrReplace {
                path,
                old_string,
                new_string,
            })
        }
        _ => None,
    }
}

pub fn parse_line(line: &str) -> Option<Event> {
    let value: Value = serde_json::from_str(line).ok()?;
    let role_user = matches!(value.get("role").and_then(Value::as_str), Some("user"));
    let content = value.get("message")?.get("content")?;

    let mut event = Event {
        role_user,
        ..Event::default()
    };
    absorb_content(&mut event, content);
    Some(event)
}

fn absorb_content(event: &mut Event, content: &Value) {
    match content {
        Value::String(text) => {
            if event.role_user {
                record_user_text(event, text);
            } else {
                record_assistant_text(event, text);
            }
        }
        Value::Array(blocks) => {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            if event.role_user {
                                record_user_text(event, text);
                            } else {
                                record_assistant_text(event, text);
                            }
                        }
                    }
                    Some("tool_use") | Some("server_tool_use") => {
                        let name = block.get("name").and_then(Value::as_str).unwrap_or_default();
                        record_tool(event, classify(name), edit_payload_len(block.get("input")));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn classify(name: &str) -> ToolKind {
    if EDIT_TOOLS.contains(&name) {
        ToolKind::Edit
    } else if READ_TOOLS.contains(&name) {
        ToolKind::Read
    } else if SHELL_TOOLS.contains(&name) {
        ToolKind::Shell
    } else {
        ToolKind::Other
    }
}

fn edit_payload_len(input: Option<&Value>) -> usize {
    let Some(input) = input else {
        return 0;
    };
    let new_string = input.get("new_string").and_then(Value::as_str);
    let contents = input.get("contents").and_then(Value::as_str);
    new_string
        .or(contents)
        .map(|text| text.len())
        .unwrap_or_default()
}

pub fn read_est_chars_from_session(path: &Path, workspace: &Path) -> io::Result<usize> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .map(|line| read_est_chars_from_line(line, workspace))
        .sum())
}

pub fn read_est_chars_from_line(line: &str, workspace: &Path) -> usize {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let Some(content) = value.get("message").and_then(|message| message.get("content")) else {
        return 0;
    };
    let mut total = 0usize;
    absorb_read_est(&mut total, content, workspace);
    total
}

fn absorb_read_est(total: &mut usize, content: &Value, workspace: &Path) {
    let Value::Array(blocks) = content else {
        return;
    };
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") | Some("server_tool_use") => {}
            _ => continue,
        };
        if block.get("name").and_then(Value::as_str) != Some("Read") {
            continue;
        }
        let Some(input) = block.get("input") else {
            continue;
        };
        let Some(path_value) = input.get("path").and_then(Value::as_str) else {
            continue;
        };
        let limit = input
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        if is_transcript_path(path_value) {
            continue;
        }
        *total += estimate_read_chars(resolve_path(workspace, path_value), limit);
    }
}

fn is_transcript_path(path_value: &str) -> bool {
    path_value.ends_with(".jsonl") && path_value.contains("agent-transcripts")
}

fn resolve_path(workspace: &Path, path_value: &str) -> PathBuf {
    let path = PathBuf::from(path_value);
    if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    }
}

fn estimate_read_chars(path: PathBuf, limit: Option<usize>) -> usize {
    let file_len = fs::metadata(&path)
        .ok()
        .map(|metadata| metadata.len() as usize)
        .unwrap_or(0);
    let cap = limit
        .map(|lines| lines.saturating_mul(AVG_LINE_CHARS))
        .unwrap_or_else(|| DEFAULT_READ_LINE_CAP.saturating_mul(AVG_LINE_CHARS));
    file_len.min(cap)
}
