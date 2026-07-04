use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::edits::EditOp;
use crate::features::{extract, Features};
use crate::scoring::{report, Report};
use crate::strictness::WeightPreset;
use crate::transcript::Event;

pub type LineParser = fn(&str) -> Option<Event>;
pub type EditLineParser = fn(&str) -> Vec<EditOp>;

pub struct SessionEngine {
    path: PathBuf,
    workspace: PathBuf,
    parse: LineParser,
    edit_parse: EditLineParser,
    read_est: fn(&str, &Path) -> usize,
    state: Arc<Mutex<Vec<Event>>>,
    edit_ops: Arc<Mutex<Vec<EditOp>>>,
    watcher: Option<RecommendedWatcher>,
}

impl SessionEngine {
    pub fn new(
        path: PathBuf,
        workspace: PathBuf,
        parse: LineParser,
        edit_parse: EditLineParser,
        read_est: fn(&str, &Path) -> usize,
    ) -> Self {
        Self {
            path,
            workspace,
            parse,
            edit_parse,
            read_est,
            state: Arc::new(Mutex::new(Vec::new())),
            edit_ops: Arc::new(Mutex::new(Vec::new())),
            watcher: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn edit_ops(&self) -> Vec<EditOp> {
        self.edit_ops
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn start(&mut self) -> notify::Result<()> {
        self.ingest();

        let path = self.path.clone();
        let workspace = self.workspace.clone();
        let parse = self.parse;
        let read_est = self.read_est;
        let edit_parse = self.edit_parse;
        let state = Arc::clone(&self.state);
        let edit_ops = Arc::clone(&self.edit_ops);
        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if event.paths.iter().any(|changed| changed == &path) {
                if let Ok((events, ops)) =
                    read_session(&path, parse, edit_parse, &workspace, read_est)
                {
                    if let Ok(mut guard) = state.lock() {
                        *guard = events;
                    }
                    if let Ok(mut guard) = edit_ops.lock() {
                        *guard = ops;
                    }
                }
            }
        })?;
        watcher.watch(&directory, RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.watcher = None;
    }

    pub fn sync_from_disk(&self) {
        self.ingest();
    }

    pub fn features(&self) -> Features {
        extract(&self.state.lock().expect("session state poisoned").clone())
    }

    pub fn report(&self) -> Report {
        report(self.features(), WeightPreset::Normal)
    }

    fn ingest(&self) {
        if let Ok((events, ops)) =
            read_session(&self.path, self.parse, self.edit_parse, &self.workspace, self.read_est)
        {
            if let Ok(mut guard) = self.state.lock() {
                *guard = events;
            }
            if let Ok(mut guard) = self.edit_ops.lock() {
                *guard = ops;
            }
        }
    }
}

fn read_session(
    path: &Path,
    parse: LineParser,
    edit_parse: EditLineParser,
    workspace: &Path,
    read_est: fn(&str, &Path) -> usize,
) -> io::Result<(Vec<Event>, Vec<EditOp>)> {
    let contents = fs::read_to_string(path)?;
    let mut events = Vec::new();
    let mut ops = Vec::new();
    for line in contents.lines() {
        ops.extend(edit_parse(line));
        if let Some(mut event) = parse(line) {
            event.read_est_chars = read_est(line, workspace);
            events.push(event);
        }
    }
    Ok((events, ops))
}
