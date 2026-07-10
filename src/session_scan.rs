use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, SystemTime};

use notify::{RecursiveMode, Watcher};

const POLL_INTERVAL: Duration = Duration::from_millis(300);

pub(crate) fn newest_session(mut dated: Vec<(SystemTime, PathBuf)>) -> Option<PathBuf> {
    dated.sort_by_key(|(modified, _)| *modified);
    dated.pop().map(|(_, path)| path)
}

pub(crate) fn find_new_session(
    root: &Path,
    known: &HashSet<PathBuf>,
    scan: fn(&Path) -> Vec<PathBuf>,
) -> Option<PathBuf> {
    scan(root).into_iter().find(|path| !known.contains(path))
}

pub(crate) fn latest_session_in(
    root: &Path,
    scan: fn(&Path) -> Vec<PathBuf>,
) -> io::Result<PathBuf> {
    let dated: Vec<(SystemTime, PathBuf)> = scan(root)
        .into_iter()
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();

    newest_session(dated).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no sessions found in {}", root.display()),
        )
    })
}

pub(crate) fn wait_for_new_session_in(
    root: &Path,
    label: &str,
    scan: fn(&Path) -> Vec<PathBuf>,
) -> io::Result<PathBuf> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{label} session directory not found: {}", root.display()),
        ));
    }

    let known: HashSet<PathBuf> = scan(root).into_iter().collect();

    let (sender, receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(to_io)?;
    watcher.watch(root, RecursiveMode::Recursive).map_err(to_io)?;

    loop {
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(event) => {
                event.map_err(to_io)?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "session watch channel closed",
                ));
            }
        }
        if let Some(found) = find_new_session(root, &known, scan) {
            return Ok(found);
        }
    }
}

fn to_io(error: notify::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
