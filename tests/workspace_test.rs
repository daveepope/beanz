use std::fs;
use std::path::PathBuf;

use beanz::workspace::{git_root, normalize_path, normalize_workspace_path, resolve_workspace};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let unique = format!(
        "beanz-workspace-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    );
    let root = std::env::temp_dir().join(unique);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn git_root_walks_up_to_git_directory() {
    let root = temp_root("git-root");
    let nested = root.join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();

    assert_eq!(git_root(&nested), Some(root.clone()));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn git_root_returns_none_without_git_directory() {
    let root = temp_root("no-git");
    let nested = root.join("only");
    fs::create_dir_all(&nested).unwrap();

    assert_eq!(git_root(&nested), None);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn resolve_workspace_prefers_beanz_path_then_build_then_git() {
    let root = temp_root("resolve");
    let beanz = root.join("beanz");
    let build = root.join("build");
    let nested = root.join("repo").join("nested");
    fs::create_dir_all(&beanz).unwrap();
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(root.join("repo").join(".git")).unwrap();

    assert_eq!(
        resolve_workspace(Some(&beanz), Some(&build), &nested),
        fs::canonicalize(&beanz).unwrap_or(beanz.clone())
    );
    assert_eq!(
        resolve_workspace(None, Some(&build), &nested),
        fs::canonicalize(&build).unwrap_or(build.clone())
    );
    assert_eq!(
        resolve_workspace(None, None, &nested),
        root.join("repo")
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn resolve_workspace_falls_back_to_cwd_without_git() {
    let root = temp_root("cwd-only");
    let nested = root.join("leaf");
    fs::create_dir_all(&nested).unwrap();

    assert_eq!(resolve_workspace(None, None, &nested), nested);

    fs::remove_dir_all(&root).ok();
}

#[test]
#[cfg(unix)]
fn normalize_path_symlink_resolves_to_target() {
    let root = temp_root("normalize");
    let readme = root.join("README.md");
    let alias = root.join("readme-alias.md");
    fs::write(&readme, "content").unwrap();
    std::os::unix::fs::symlink(&readme, &alias).unwrap();

    assert_eq!(normalize_path(&alias), normalize_path(&readme));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn normalize_workspace_path_resolves_relative_under_root() {
    let root = temp_root("workspace-relative");
    let readme = root.join("README.md");
    fs::write(&readme, "content").unwrap();

    assert_eq!(
        normalize_workspace_path(&root, PathBuf::from("README.md")),
        normalize_path(&readme)
    );

    fs::remove_dir_all(&root).ok();
}
