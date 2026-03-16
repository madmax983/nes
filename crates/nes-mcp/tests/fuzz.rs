use nes_mcp::{dispatch_tool, ToolParams};
use nes_core::NesCore;
use std::fs;

#[test]
fn havoc_test_fuzz_dispatch_capture_frame_path_traversal() {
    let mut core = NesCore::new();
    let mut params = ToolParams::new();

    // We expect the path to be invalid or file to not exist, but we MUST NOT PANIC.
    // However, if we can do path traversal, we might write files wherever we want.
    // We will attempt to write a file to a temporary directory using traversal.
    let temp_dir = std::env::temp_dir();
    let malicious_path = format!("{}/test_traversal.bmp", temp_dir.display());

    // Construct a path that traverses from the current working dir.
    // E.g. "../../../../../tmp/test_traversal.bmp"
    let pwd = std::env::current_dir().unwrap();
    let mut dots = String::new();
    for _ in 0..pwd.components().count() {
        dots.push_str("../");
    }
    let traversal_path = format!("{}{}", dots, malicious_path.trim_start_matches('/'));

    params.insert("path".to_string(), traversal_path.clone());

    let result = dispatch_tool(&mut core, "capture_frame", &params);
    println!("{:?}", result);

    if result.is_ok() {
        let _ = fs::remove_file(&malicious_path);
    }

    // As a Havoc test, we'll assert that it *should* fail if the system is secure.
    // But since it passes, the test itself fails, proving the vulnerability.
    assert!(result.is_err(), "VULNERABILITY: capture_frame allows arbitrary path writes via directory traversal");
}
