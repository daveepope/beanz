#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let line = String::from_utf8_lossy(data);
    let workspace = Path::new("/tmp");
    let _ = beanz::claude::read_est_chars_from_line(&line, workspace);
});
