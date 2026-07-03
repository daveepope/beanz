use std::fs;

use beanz::workspace::git_root;

#[test]
fn git_root_walks_up_to_git_directory() {
    let unique = format!(
        "beanz-git-root-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    );
    let root = std::env::temp_dir().join(unique);
    let nested = root.join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();

    assert_eq!(git_root(&nested), Some(root.clone()));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn git_root_returns_none_without_git_directory() {
    let unique = format!(
        "beanz-no-git-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    );
    let root = std::env::temp_dir().join(unique);
    let nested = root.join("only");
    fs::create_dir_all(&nested).unwrap();

    assert_eq!(git_root(&nested), None);

    fs::remove_dir_all(&root).ok();
}
