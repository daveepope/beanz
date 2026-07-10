use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::complexity::is_source_path;
use crate::edits::EditOp;
use crate::transcript::{record_assistant_text, record_tool, record_user_text, Event, ToolKind};

const EDIT_TOOLS: [&str; 3] = ["Write", "Edit", "MultiEdit"];
const READ_TOOLS: [&str; 3] = ["Read", "Grep", "Glob"];
const SHELL_TOOLS: [&str; 1] = ["Bash"];
const DEFAULT_READ_LINE_CAP: usize = 50;
const AVG_LINE_CHARS: usize = 80;

fn is_sidechain(value: &Value) -> bool {
    value
        .get("isSidechain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn role_user(value: &Value) -> Option<bool> {
    match value.get("type").and_then(Value::as_str)? {
        "user" => Some(true),
        "assistant" => Some(false),
        _ => None,
    }
}

fn decode_line(line: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(line).ok()?;
    if is_sidechain(&value) {
        return None;
    }
    Some(value)
}

fn is_tool_result_only(content: &Value) -> bool {
    match content {
        Value::Array(blocks) if !blocks.is_empty() => blocks
            .iter()
            .all(|block| block.get("type").and_then(Value::as_str) == Some("tool_result")),
        _ => false,
    }
}

pub fn edit_ops_from_line(line: &str) -> Vec<EditOp> {
    let Some(value) = decode_line(line) else {
        return Vec::new();
    };
    let Some(content) = value.get("message").and_then(|message| message.get("content")) else {
        return Vec::new();
    };
    let mut ops = Vec::new();
    absorb_edit_ops(&mut ops, content);
    ops
}

fn absorb_edit_ops(ops: &mut Vec<EditOp>, content: &Value) {
    let Value::Array(blocks) = content else {
        return;
    };
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") | Some("server_tool_use") => {}
            _ => continue,
        }
        let name = block.get("name").and_then(Value::as_str).unwrap_or_default();
        if let Some(input) = block.get("input") {
            edit_ops_from_input(ops, name, input);
        }
    }
}

fn edit_ops_from_input(ops: &mut Vec<EditOp>, name: &str, input: &Value) {
    let Some(path_str) = input.get("file_path").and_then(Value::as_str) else {
        return;
    };
    let path = PathBuf::from(path_str);
    match name {
        "Write" => {
            if let Some(contents) = input.get("content").and_then(Value::as_str) {
                ops.push(EditOp::Write {
                    path,
                    contents: contents.to_string(),
                });
            }
        }
        "Edit" => {
            if let Some(op) = str_replace_op(&path, input) {
                ops.push(op);
            }
        }
        "MultiEdit" => {
            let Some(edits) = input.get("edits").and_then(Value::as_array) else {
                return;
            };
            for edit in edits {
                if let Some(op) = str_replace_op(&path, edit) {
                    ops.push(op);
                }
            }
        }
        _ => {}
    }
}

fn str_replace_op(path: &Path, input: &Value) -> Option<EditOp> {
    let old_string = input.get("old_string").and_then(Value::as_str)?.to_string();
    let new_string = input.get("new_string").and_then(Value::as_str)?.to_string();
    Some(EditOp::StrReplace {
        path: path.to_path_buf(),
        old_string,
        new_string,
    })
}

pub fn parse_line(line: &str) -> Option<Event> {
    let value = decode_line(line)?;
    let is_user = role_user(&value)?;
    let content = value.get("message")?.get("content")?;
    if is_user && is_tool_result_only(content) {
        return None;
    }

    let mut event = Event {
        role_user: is_user,
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
                        let input = block.get("input");
                        record_tool(
                            event,
                            classify(name),
                            edit_payload_len(input),
                            is_code_target(input),
                        );
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn is_code_target(input: Option<&Value>) -> bool {
    input
        .and_then(|input| input.get("file_path"))
        .and_then(Value::as_str)
        .map(|path| is_source_path(Path::new(path)))
        .unwrap_or(true)
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
    if let Some(edits) = input.get("edits").and_then(Value::as_array) {
        return edits
            .iter()
            .filter_map(|edit| edit.get("new_string").and_then(Value::as_str))
            .map(str::len)
            .sum();
    }
    let new_string = input.get("new_string").and_then(Value::as_str);
    let content = input.get("content").and_then(Value::as_str);
    new_string
        .or(content)
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
    let Some(value) = decode_line(line) else {
        return 0;
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
        let Some(path_value) = input.get("file_path").and_then(Value::as_str) else {
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
    path_value.ends_with(".jsonl") && path_value.contains(".claude/projects")
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
