use std::path::{Path, PathBuf};

pub fn workspace_root() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("BEANZ_WORKSPACE") {
        let path = PathBuf::from(value);
        if path.is_dir() {
            return Some(canonicalize_lossy(path));
        }
    }
    if let Ok(value) = std::env::var("BUILD_WORKSPACE_DIRECTORY") {
        let path = PathBuf::from(value);
        if path.is_dir() {
            return Some(canonicalize_lossy(path));
        }
    }
    let cwd = std::env::current_dir().ok()?;
    git_root(&cwd).or(Some(cwd))
}

pub fn git_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn canonicalize_lossy(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}
