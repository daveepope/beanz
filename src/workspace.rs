use std::path::{Path, PathBuf};

pub fn resolve_workspace(
    beanz_workspace: Option<&Path>,
    build_workspace: Option<&Path>,
    cwd: &Path,
) -> PathBuf {
    if let Some(path) = beanz_workspace {
        if path.is_dir() {
            return canonicalize_lossy(path.to_path_buf());
        }
    }
    if let Some(path) = build_workspace {
        if path.is_dir() {
            return canonicalize_lossy(path.to_path_buf());
        }
    }
    git_root(cwd).unwrap_or_else(|| cwd.to_path_buf())
}

pub fn workspace_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    Some(resolve_workspace(
        std::env::var("BEANZ_WORKSPACE")
            .ok()
            .map(PathBuf::from)
            .as_deref(),
        std::env::var("BUILD_WORKSPACE_DIRECTORY")
            .ok()
            .map(PathBuf::from)
            .as_deref(),
        &cwd,
    ))
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
