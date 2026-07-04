use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::complexity::{
    baseline_bytes, baseline_complexity, collect_source_files, complexity_of_source, Language,
};
use crate::edits::EditOp;

pub struct ScoreMaps {
    pub baseline: HashMap<PathBuf, u32>,
    pub current: HashMap<PathBuf, u32>,
    pub baseline_bytes: HashMap<PathBuf, u64>,
    pub current_bytes: HashMap<PathBuf, u64>,
}

pub fn reconstruct_baseline(
    root: &Path,
    session_start: SystemTime,
    edit_ops: &[EditOp],
) -> ScoreMaps {
    let current = baseline_complexity(root);
    let current_bytes = baseline_bytes(root);

    let mut touched = HashSet::new();
    for op in edit_ops {
        let resolved = resolve_path(root, op.path());
        if Language::from_path(&resolved).is_some() {
            touched.insert(resolved);
        }
    }

    let mut replayed: HashMap<PathBuf, Option<String>> = HashMap::new();
    for path in &touched {
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

    let mut baseline = HashMap::new();
    let mut baseline_bytes_map = HashMap::new();

    for path in collect_source_files(root) {
        if touched.contains(&path) {
            continue;
        }
        if modified_before_session(&path, session_start) {
            if let Some(&value) = current.get(&path) {
                baseline.insert(path.clone(), value);
            }
            if let Some(&size) = current_bytes.get(&path) {
                baseline_bytes_map.insert(path.clone(), size);
            }
        }
    }

    for path in &touched {
        match replayed.get(path) {
            Some(Some(content)) => {
                baseline_bytes_map.insert(path.clone(), content.len() as u64);
                if let Some(value) = complexity_of_source(content, path) {
                    baseline.insert(path.clone(), value);
                }
            }
            Some(None) | None => {}
        }
    }

    ScoreMaps {
        baseline,
        current,
        baseline_bytes: baseline_bytes_map,
        current_bytes,
    }
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn modified_before_session(path: &Path, session_start: SystemTime) -> bool {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(|modified| modified < session_start)
        .unwrap_or(false)
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
                if let Some(pos) = text.find(new_string.as_str()) {
                    let end = pos + new_string.len();
                    text.replace_range(pos..end, old_string.as_str());
                }
            }
        }
    }
}
