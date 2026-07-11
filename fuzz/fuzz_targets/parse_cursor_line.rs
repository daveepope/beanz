#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let line = String::from_utf8_lossy(data);
    let _ = beanz::cursor::parse_line(&line);
    let _ = beanz::cursor::edit_ops_from_line(&line);
});
