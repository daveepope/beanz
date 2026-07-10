#![no_main]

use beanz::complexity::{cyclomatic, Language};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    let _ = cyclomatic(&source, Language::Java);
});
