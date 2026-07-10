use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::complexity::{complexity_of, complexity_of_source, file_bytes};
use crate::edits::EditOp;

pub struct ScoreMaps {
    pub baseline: HashMap<PathBuf, u32>,
    pub current: HashMap<PathBuf, u32>,
    pub baseline_bytes: HashMap<PathBuf, u64>,
    pub current_bytes: HashMap<PathBuf, u64>,
}

pub fn reconstruct_baseline(
    root: &Path,
    edit_ops: &[EditOp],
    touched: &HashSet<PathBuf>,
    session_open: &HashMap<PathBuf, u32>,
    session_open_bytes: &HashMap<PathBuf, u64>,
) -> ScoreMaps {
    if touched.is_empty() {
        return ScoreMaps {
            baseline: HashMap::new(),
            current: HashMap::new(),
            baseline_bytes: HashMap::new(),
            current_bytes: HashMap::new(),
        };
    }

    let transcript_paths = touched_from_edit_ops(root, edit_ops);
    let replayed = replay_baseline(root, edit_ops, &transcript_paths);
    let mut baseline = HashMap::new();
    let mut baseline_bytes_map = HashMap::new();

    for path in touched {
        if transcript_paths.contains(path) {
            if let Some(content) = replayed.get(path).and_then(|value| value.as_ref()) {
                baseline_bytes_map.insert(path.clone(), content.len() as u64);
                if let Some(value) = complexity_of_source(content, path) {
                    baseline.insert(path.clone(), value);
                }
            } else if let Some(&value) = session_open.get(path) {
                baseline.insert(path.clone(), value);
                if let Some(&size) = session_open_bytes.get(path) {
                    baseline_bytes_map.insert(path.clone(), size);
                }
            }
        } else if let Some(&value) = session_open.get(path) {
            baseline.insert(path.clone(), value);
            if let Some(&size) = session_open_bytes.get(path) {
                baseline_bytes_map.insert(path.clone(), size);
            }
        }
    }

    ScoreMaps {
        baseline,
        current: current_for_paths(touched),
        baseline_bytes: baseline_bytes_map,
        current_bytes: bytes_for_paths(touched),
    }
}

fn replay_baseline(
    root: &Path,
    edit_ops: &[EditOp],
    touched: &HashSet<PathBuf>,
) -> HashMap<PathBuf, Option<String>> {
    let mut replayed: HashMap<PathBuf, Option<String>> = HashMap::new();
    for path in touched {
        replayed.insert(path.clone(), fs::read_to_string(path).ok());
    }

    for op in edit_ops.iter().rev() {
        let path = resolve_path(root, op.path());
        if !touched.contains(&path) {
            continue;
        }
        let entry = replayed.entry(path).or_insert(None);
        reverse_apply(entry, op);
    }
    replayed
}

fn current_for_paths(paths: &HashSet<PathBuf>) -> HashMap<PathBuf, u32> {
    paths
        .iter()
        .filter(|path| path.is_file())
        .filter_map(|path| complexity_of(path).map(|value| (path.clone(), value)))
        .collect()
}

fn bytes_for_paths(paths: &HashSet<PathBuf>) -> HashMap<PathBuf, u64> {
    paths
        .iter()
        .filter_map(|path| path.is_file().then(|| (path.clone(), file_bytes(path))))
        .collect()
}

pub fn touched_from_edit_ops(root: &Path, edit_ops: &[EditOp]) -> HashSet<PathBuf> {
    let mut touched = HashSet::new();
    for op in edit_ops {
        touched.insert(resolve_path(root, op.path()));
    }
    touched
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn reverse_apply(content: &mut Option<String>, op: &EditOp) {
    match op {
        EditOp::Write { contents, .. } => {
            if content.as_deref() == Some(contents.as_str()) {
                *content = None;
            }
        }
        EditOp::StrReplace {
            old_string,
            new_string,
            ..
        } => {
            if let Some(text) = content {
                *text = text.replace(new_string.as_str(), old_string.as_str());
            }
        }
    }
}
