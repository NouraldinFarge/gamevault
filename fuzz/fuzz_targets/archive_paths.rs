#![no_main]

use libfuzzer_sys::fuzz_target;

#[allow(dead_code)]
#[path = "../../src-tauri/src/archive_paths.rs"]
mod archive_paths;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = std::str::from_utf8(data) else {
        return;
    };

    let _ = archive_paths::parse_listing(value);
    let _ = archive_paths::unsafe_archive_path(value);
    let _ = archive_paths::normalized_archive_key(value);
});
