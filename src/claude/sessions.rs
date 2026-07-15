use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn session_env() -> Result<(PathBuf, PathBuf), io::Error> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve Claude session directory for this workspace",
        )
    })?;
    let workspace = crate::workspace::workspace_root().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve Claude session directory for this workspace",
        )
    })?;
    Ok((PathBuf::from(home), workspace))
}

pub fn transcripts_root() -> Option<PathBuf> {
    let (home, workspace) = session_env().ok()?;
    Some(session_root(&home, &workspace))
}

pub fn session_root(home: &Path, workspace: &Path) -> PathBuf {
    home.join(".claude")
        .join("projects")
        .join(workspace_slug(workspace))
}

fn workspace_slug(workspace: &Path) -> String {
    crate::workspace::slug_path(workspace, false, &['.'])
}

pub fn scan_sessions(root: &Path) -> Vec<PathBuf> {
    let mut sessions = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return sessions;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            sessions.push(path);
        }
    }
    sessions
}

pub fn find_new_session(root: &Path, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    crate::session_scan::find_new_session(root, known, scan_sessions)
}

pub fn newest_session(dated: Vec<(SystemTime, PathBuf)>) -> Option<PathBuf> {
    crate::session_scan::newest_session(dated)
}

pub fn latest_session_at(home: &Path, workspace: &Path) -> io::Result<PathBuf> {
    latest_session_in(&session_root(home, workspace))
}

pub fn wait_for_new_session_at(home: &Path, workspace: &Path) -> io::Result<PathBuf> {
    wait_for_new_session_in(&session_root(home, workspace))
}

pub fn latest_session() -> io::Result<PathBuf> {
    let (home, workspace) = session_env()?;
    latest_session_at(&home, &workspace)
}

pub fn latest_session_in(root: &Path) -> io::Result<PathBuf> {
    crate::session_scan::latest_session_in(root, scan_sessions)
}

pub fn wait_for_new_session() -> io::Result<PathBuf> {
    let (home, workspace) = session_env()?;
    wait_for_new_session_at(&home, &workspace)
}

pub fn wait_for_new_session_in(root: &Path) -> io::Result<PathBuf> {
    crate::session_scan::wait_for_new_session_in(root, "Claude", scan_sessions)
}

