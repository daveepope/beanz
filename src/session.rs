use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::cursor::transcript::{edit_ops_from_line, EditOp};
use crate::features::{extract, Features};
use crate::scoring::{report, Report};
use crate::transcript::Event;

pub type LineParser = fn(&str) -> Option<Event>;

pub struct SessionEngine {
    path: PathBuf,
    parse: LineParser,
    state: Arc<Mutex<Vec<Event>>>,
    watcher: Option<RecommendedWatcher>,
}

impl SessionEngine {
    pub fn new(path: PathBuf, parse: LineParser) -> Self {
        Self {
            path,
            parse,
            state: Arc::new(Mutex::new(Vec::new())),
            watcher: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn prepare(&mut self) {
        self.ingest();
    }

    pub fn edit_ops(&self) -> Vec<EditOp> {
        read_edit_ops(&self.path).unwrap_or_default()
    }

    pub fn created(&self) -> SystemTime {
        fs::metadata(&self.path)
            .ok()
            .and_then(|metadata| metadata.created().ok())
            .or_else(|| {
                fs::metadata(&self.path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
            })
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    pub fn start(&mut self) -> notify::Result<()> {
        self.ingest();

        let path = self.path.clone();
        let parse = self.parse;
        let state = Arc::clone(&self.state);
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
                if let Ok(events) = read_events(&path, parse) {
                    if let Ok(mut guard) = state.lock() {
                        *guard = events;
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

    pub fn features(&self) -> Features {
        let events = match read_events(&self.path, self.parse) {
            Ok(events) => events,
            Err(_) => self.state.lock().expect("session state poisoned").clone(),
        };
        if let Ok(mut guard) = self.state.lock() {
            *guard = events.clone();
        }
        extract(&events)
    }

    pub fn report(&self) -> Report {
        report(self.features())
    }

    fn ingest(&self) {
        if let Ok(events) = read_events(&self.path, self.parse) {
            if let Ok(mut guard) = self.state.lock() {
                *guard = events;
            }
        }
    }
}

fn read_edit_ops(path: &Path) -> io::Result<Vec<EditOp>> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .flat_map(edit_ops_from_line)
        .collect())
}

fn read_events(path: &Path, parse: LineParser) -> io::Result<Vec<Event>> {
    let contents = fs::read_to_string(path)?;
    Ok(contents.lines().filter_map(parse).collect())
}
