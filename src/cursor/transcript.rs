use std::path::PathBuf;

use serde_json::Value;

use crate::transcript::{record_tool, record_user_text, Event, ToolKind};

const EDIT_TOOLS: [&str; 3] = ["Write", "StrReplace", "Edit"];
const READ_TOOLS: [&str; 3] = ["Read", "Grep", "Glob"];
const SHELL_TOOLS: [&str; 2] = ["Shell", "Bash"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOp {
    Write {
        path: PathBuf,
        contents: String,
    },
    StrReplace {
        path: PathBuf,
        old_string: String,
        new_string: String,
    },
}

impl EditOp {
    pub fn path(&self) -> &PathBuf {
        match self {
            EditOp::Write { path, .. } | EditOp::StrReplace { path, .. } => path,
        }
    }
}

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
            }
        }
        Value::Array(blocks) => {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if event.role_user {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                record_user_text(event, text);
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
