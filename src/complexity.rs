use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tree_sitter::{Node, Parser};

use crate::edits::EditOp;
use crate::score_snapshot::{reconstruct_baseline, touched_from_edit_ops};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    Java,
}

const RECOGNIZED_SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "py", "java", "js", "jsx", "mjs", "cjs", "ts", "tsx", "go", "c", "h", "cpp", "cc",
    "cxx", "hpp", "hh", "cs", "rb", "php", "swift", "kt", "kts", "scala", "m", "mm", "sh",
    "bash", "zsh", "pl", "lua",
];

pub fn is_source_extension(extension: &str) -> bool {
    RECOGNIZED_SOURCE_EXTENSIONS.contains(&extension) || Language::from_extension(extension).is_some()
}

pub fn is_source_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(is_source_extension)
        .unwrap_or(false)
}

impl Language {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(Self::from_extension)
    }

    fn grammar(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::Java => tree_sitter_java::LANGUAGE.into(),
        }
    }

    fn decision_kinds(self) -> &'static [&'static str] {
        match self {
            Language::Rust => &[
                "if_expression",
                "match_arm",
                "while_expression",
                "for_expression",
                "loop_expression",
                "&&",
                "||",
            ],
            Language::Python => &[
                "if_statement",
                "elif_clause",
                "for_statement",
                "while_statement",
                "except_clause",
                "conditional_expression",
                "case_clause",
                "and",
                "or",
            ],
            Language::Java => &[
                "if_statement",
                "while_statement",
                "for_statement",
                "enhanced_for_statement",
                "do_statement",
                "catch_clause",
                "switch_label",
                "ternary_expression",
                "&&",
                "||",
            ],
        }
    }
}

pub fn cyclomatic(source: &str, language: Language) -> u32 {
    let mut parser = Parser::new();
    if parser.set_language(&language.grammar()).is_err() {
        return 1;
    }
    let Some(tree) = parser.parse(source, None) else {
        return 1;
    };
    1 + count_decisions(tree.root_node(), language)
}

fn count_decisions(node: Node, language: Language) -> u32 {
    let mut total = if language.decision_kinds().contains(&node.kind()) {
        1
    } else {
        0
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        total += count_decisions(child, language);
    }
    total
}

pub fn complexity_of_source(source: &str, path: &Path) -> Option<u32> {
    Language::from_path(path).map(|language| cyclomatic(source, language))
}

pub fn complexity_of(path: &Path) -> Option<u32> {
    let source = fs::read_to_string(path).ok()?;
    complexity_of_source(&source, path)
}

const IGNORED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "__pycache__",
    "dist",
    "build",
    "out",
];

pub fn collect_all_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !is_ignored_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

pub fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    collect_all_files(root)
        .into_iter()
        .filter(|path| Language::from_path(path).is_some())
        .collect()
}

fn is_ignored_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    is_ignored_name(name)
}

fn is_ignored_name(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("bazel-") || IGNORED_DIRS.contains(&name)
}

fn is_ignored_descendant(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(is_ignored_name)
            .unwrap_or(true)
    })
}

pub fn watchable_directories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_directory = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if is_directory && !is_ignored_dir(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    directories.sort();
    directories
}

pub fn baseline_complexity(root: &Path) -> HashMap<PathBuf, u32> {
    parallel_complexity(collect_source_files(root))
}

pub fn file_bytes(path: &Path) -> u64 {
    fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0)
}

pub fn baseline_bytes(root: &Path) -> HashMap<PathBuf, u64> {
    collect_all_files(root)
        .into_iter()
        .map(|path| (path.clone(), file_bytes(&path)))
        .collect()
}

pub fn bytes_delta(baseline: &HashMap<PathBuf, u64>, current: &HashMap<PathBuf, u64>) -> i64 {
    let baseline_total: i64 = baseline.values().map(|size| *size as i64).sum();
    let current_total: i64 = current.values().map(|size| *size as i64).sum();
    current_total - baseline_total
}

pub fn files_delta(baseline: &HashMap<PathBuf, u64>, current: &HashMap<PathBuf, u64>) -> i64 {
    current.len() as i64 - baseline.len() as i64
}

fn parallel_complexity(files: Vec<PathBuf>) -> HashMap<PathBuf, u32> {
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(files.len());

    if workers <= 1 {
        return files
            .into_iter()
            .filter_map(|path| complexity_of(&path).map(|value| (path, value)))
            .collect();
    }

    let chunk_size = files.len().div_ceil(workers);
    let mut result = HashMap::with_capacity(files.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|path| complexity_of(path).map(|value| (path.clone(), value)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for handle in handles {
            if let Ok(partial) = handle.join() {
                result.extend(partial);
            }
        }
    });
    result
}

fn total(map: &HashMap<PathBuf, u32>) -> u32 {
    map.values().copied().sum()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComplexityDelta {
    Complexity {
        path: PathBuf,
        baseline: u32,
        current: u32,
    },
    Bytes {
        path: PathBuf,
        baseline: u64,
        current: u64,
    },
}

impl ComplexityDelta {
    pub fn path(&self) -> &Path {
        match self {
            ComplexityDelta::Complexity { path, .. } => path,
            ComplexityDelta::Bytes { path, .. } => path,
        }
    }

    pub fn delta(&self) -> i64 {
        match self {
            ComplexityDelta::Complexity {
                baseline, current, ..
            } => i64::from(*current) - i64::from(*baseline),
            ComplexityDelta::Bytes {
                baseline, current, ..
            } => *current as i64 - *baseline as i64,
        }
    }
}

pub fn compute_deltas(
    baseline: &HashMap<PathBuf, u32>,
    current: &HashMap<PathBuf, u32>,
    baseline_bytes: &HashMap<PathBuf, u64>,
    current_bytes: &HashMap<PathBuf, u64>,
) -> Vec<ComplexityDelta> {
    let mut deltas: Vec<ComplexityDelta> = Vec::new();
    let mut scored: HashSet<PathBuf> = HashSet::new();
    for (path, &current_value) in current {
        scored.insert(path.clone());
        let baseline_value = baseline.get(path).copied().unwrap_or(0);
        if current_value != baseline_value {
            deltas.push(ComplexityDelta::Complexity {
                path: path.clone(),
                baseline: baseline_value,
                current: current_value,
            });
        }
    }
    for (path, &baseline_value) in baseline {
        scored.insert(path.clone());
        if !current.contains_key(path) {
            deltas.push(ComplexityDelta::Complexity {
                path: path.clone(),
                baseline: baseline_value,
                current: 0,
            });
        }
    }

    let mut byte_paths: HashSet<&PathBuf> = HashSet::new();
    byte_paths.extend(baseline_bytes.keys());
    byte_paths.extend(current_bytes.keys());
    for path in byte_paths {
        if scored.contains(path) {
            continue;
        }
        let baseline_value = baseline_bytes.get(path).copied().unwrap_or(0);
        let current_value = current_bytes.get(path).copied().unwrap_or(0);
        if current_value != baseline_value {
            deltas.push(ComplexityDelta::Bytes {
                path: path.clone(),
                baseline: baseline_value,
                current: current_value,
            });
        }
    }

    deltas.sort_by(|left, right| {
        right
            .delta()
            .abs()
            .cmp(&left.delta().abs())
            .then_with(|| left.path().cmp(right.path()))
    });
    deltas
}

#[derive(Default)]
struct DiskState {
    session_open: HashMap<PathBuf, u32>,
    session_open_bytes: HashMap<PathBuf, u64>,
    baseline: HashMap<PathBuf, u32>,
    current: HashMap<PathBuf, u32>,
    baseline_bytes: HashMap<PathBuf, u64>,
    current_bytes: HashMap<PathBuf, u64>,
    disk_touched: HashSet<PathBuf>,
}

pub struct ComplexityEngine {
    root: PathBuf,
    state: Arc<Mutex<DiskState>>,
    watcher: Option<RecommendedWatcher>,
}

impl ComplexityEngine {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Arc::new(Mutex::new(DiskState::default())),
            watcher: None,
        }
    }

    pub fn start(&mut self) -> notify::Result<()> {
        let complexity = baseline_complexity(&self.root);
        let bytes = baseline_bytes(&self.root);
        if let Ok(mut guard) = self.state.lock() {
            guard.session_open = complexity;
            guard.session_open_bytes = bytes;
        }
        self.attach_watcher()
    }

    pub fn sync_from_session(&self, edit_ops: &[EditOp]) {
        let maps = {
            let guard = self.state.lock().expect("complexity state poisoned");
            let mut touched = touched_from_edit_ops(&self.root, edit_ops);
            touched.extend(guard.disk_touched.iter().cloned());
            touched.extend(paths_changed_since_open(
                &self.root,
                &guard.session_open_bytes,
            ));
            reconstruct_baseline(
                &self.root,
                edit_ops,
                &touched,
                &guard.session_open,
                &guard.session_open_bytes,
            )
        };
        if let Ok(mut guard) = self.state.lock() {
            guard.baseline = maps.baseline;
            guard.current = maps.current;
            guard.baseline_bytes = maps.baseline_bytes;
            guard.current_bytes = maps.current_bytes;
        }
    }

    fn attach_watcher(&mut self) -> notify::Result<()> {
        let state = Arc::clone(&self.state);
        let root = self.root.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    return;
                };
                for path in event.paths {
                    if is_ignored_descendant(&root, &path) {
                        continue;
                    }
                    if let Ok(mut guard) = state.lock() {
                        guard.disk_touched.insert(path);
                    }
                }
            })?;

        let _ = watcher.watch(&self.root, RecursiveMode::NonRecursive);
        for directory in watchable_directories(&self.root) {
            let _ = watcher.watch(&directory, RecursiveMode::Recursive);
        }
        self.watcher = Some(watcher);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.watcher = None;
    }

    pub fn introduced(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        i64::from(total(&guard.current)) - i64::from(total(&guard.baseline))
    }

    pub fn bytes_delta(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        bytes_delta(&guard.baseline_bytes, &guard.current_bytes)
    }

    pub fn files_delta(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        let baseline_count = guard
            .baseline_bytes
            .keys()
            .filter(|path| Language::from_path(path).is_some())
            .count() as i64;
        let current_count = guard
            .current_bytes
            .keys()
            .filter(|path| Language::from_path(path).is_some())
            .count() as i64;
        current_count - baseline_count
    }

    pub fn deltas(&self) -> Vec<ComplexityDelta> {
        let guard = self.state.lock().expect("complexity state poisoned");
        compute_deltas(
            &guard.baseline,
            &guard.current,
            &guard.baseline_bytes,
            &guard.current_bytes,
        )
    }
}

fn paths_changed_since_open(
    root: &Path,
    session_open_bytes: &HashMap<PathBuf, u64>,
) -> HashSet<PathBuf> {
    let mut changed = HashSet::new();
    let current_files: HashSet<PathBuf> = collect_all_files(root).into_iter().collect();

    for path in &current_files {
        let size = file_bytes(path);
        if session_open_bytes.get(path) != Some(&size) {
            changed.insert(path.clone());
        }
    }
    for path in session_open_bytes.keys() {
        if !current_files.contains(path) {
            changed.insert(path.clone());
        }
    }
    changed
}
