#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let workspace = Path::new("/tmp");
    let path = workspace.join("beanz-fuzz-cursor-session");
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let _ = beanz::cursor::read_est_chars_from_session(&path, workspace);
    let _ = std::fs::remove_file(path);
});
