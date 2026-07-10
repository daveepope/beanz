#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use beanz::AgentHarness;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct HarnessFactory {
    harness: AgentHarness,
}

impl HarnessFactory {
    pub fn new(harness: AgentHarness) -> Self {
        Self { harness }
    }

    pub fn cursor() -> Self {
        Self::new(AgentHarness::Cursor)
    }

    pub fn claude() -> Self {
        Self::new(AgentHarness::Claude)
    }

    pub fn harness(&self) -> AgentHarness {
        self.harness
    }

    pub fn user_line(&self, text: &str) -> String {
        render_user(self.harness, text)
    }

    pub fn tool_call_line(&self, name: &str, input: &str) -> String {
        render_tool_call(self.harness, name, input)
    }

    pub fn str_replace_line(&self, path: &str, old_string: &str, new_string: &str) -> String {
        render_str_replace(self.harness, path, old_string, new_string)
    }

    pub fn write_at_line(&self, path: &str, contents: &str) -> String {
        render_write_at(self.harness, path, contents)
    }

    pub fn session(&self) -> Session {
        Session {
            harness: self.harness,
            lines: Vec::new(),
        }
    }
}

pub struct Session {
    harness: AgentHarness,
    lines: Vec<String>,
}

impl Session {
    pub fn user(mut self, text: &str) -> Self {
        self.lines.push(render_user(self.harness, text));
        self
    }

    pub fn tool_call(mut self, name: &str, input: &str) -> Self {
        self.lines.push(render_tool_call(self.harness, name, input));
        self
    }

    pub fn str_replace(mut self, path: &str, old_string: &str, new_string: &str) -> Self {
        self.lines
            .push(render_str_replace(self.harness, path, old_string, new_string));
        self
    }

    pub fn write_at(mut self, path: &str, contents: &str) -> Self {
        self.lines.push(render_write_at(self.harness, path, contents));
        self
    }

    pub fn write(mut self, bytes: usize) -> Self {
        self.lines.push(render_write(self.harness, bytes));
        self
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.clone()
    }

    pub fn to_file(&self) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("beanz_{}_{}.jsonl", std::process::id(), unique));
        std::fs::write(&path, self.lines.join("\n")).unwrap();
        path
    }
}

fn render_user(harness: AgentHarness, text: &str) -> String {
    let encoded = serde_json::to_string(text).unwrap();
    match harness {
        AgentHarness::Cursor => format!(
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":{encoded}}}]}}}}"#
        ),
        AgentHarness::Claude => format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{encoded}}}]}}}}"#
        ),
    }
}

fn render_tool_call(harness: AgentHarness, name: &str, input: &str) -> String {
    match harness {
        AgentHarness::Cursor => format!(
            r#"{{"role":"assistant","message":{{"content":[{{"type":"tool_use","name":"{name}","input":{input}}}]}}}}"#
        ),
        AgentHarness::Claude => format!(
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"tool_use","name":"{name}","input":{input}}}]}}}}"#
        ),
    }
}

fn render_write(harness: AgentHarness, bytes: usize) -> String {
    let payload = "x".repeat(bytes);
    match harness {
        AgentHarness::Cursor => {
            render_tool_call(harness, "Write", &format!(r#"{{"contents":"{payload}"}}"#))
        }
        AgentHarness::Claude => {
            render_tool_call(harness, "Write", &format!(r#"{{"content":"{payload}"}}"#))
        }
    }
}

fn render_write_at(harness: AgentHarness, path: &str, contents: &str) -> String {
    let encoded_path = serde_json::to_string(path).unwrap();
    let encoded_contents = serde_json::to_string(contents).unwrap();
    match harness {
        AgentHarness::Cursor => render_tool_call(
            harness,
            "Write",
            &format!(r#"{{"path":{encoded_path},"contents":{encoded_contents}}}"#),
        ),
        AgentHarness::Claude => render_tool_call(
            harness,
            "Write",
            &format!(r#"{{"file_path":{encoded_path},"content":{encoded_contents}}}"#),
        ),
    }
}

fn render_str_replace(
    harness: AgentHarness,
    path: &str,
    old_string: &str,
    new_string: &str,
) -> String {
    let encoded_path = serde_json::to_string(path).unwrap();
    let old_string = serde_json::to_string(old_string).unwrap();
    let new_string = serde_json::to_string(new_string).unwrap();
    match harness {
        AgentHarness::Cursor => render_tool_call(
            harness,
            "StrReplace",
            &format!(
                r#"{{"path":{encoded_path},"old_string":{old_string},"new_string":{new_string}}}"#
            ),
        ),
        AgentHarness::Claude => render_tool_call(
            harness,
            "Edit",
            &format!(
                r#"{{"file_path":{encoded_path},"old_string":{old_string},"new_string":{new_string}}}"#
            ),
        ),
    }
}
