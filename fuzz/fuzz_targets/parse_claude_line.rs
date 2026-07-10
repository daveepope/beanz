#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let line = String::from_utf8_lossy(data);
    let _ = beanz::claude::parse_line(&line);
    let _ = beanz::claude::edit_ops_from_line(&line);
});
