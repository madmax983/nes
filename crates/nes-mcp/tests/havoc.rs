use nes_core::NesCore;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;
use std::fs;
use std::path::Path;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    #[ignore = "Havoc Path Traversal Attack"]
    fn havoc_proptest_capture_frame_path_traversal(
        // Generate strings that might look like absolute paths or traversals
        path in r"(/tmp/|C:\\temp\\|\.\\|\.\./|\.\.\\)[a-zA-Z0-9_]+\.ppm"
    ) {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();

        // Clean up before attack
        let _ = fs::remove_file(&path);

        params.insert("path".to_owned(), path.clone());

        // Trigger the arbitrary file write via public API
        let result = dispatch_tool(&mut core, "capture_frame", &params);

        // If the tool succeeded, we check if the file was actually written to the filesystem.
        // If it was, the system is fragile because it allowed arbitrary paths!
        if result.is_ok() {
            let written = Path::new(&path).exists();

            // Clean up after attack
            let _ = fs::remove_file(&path);

            assert!(
                !written,
                "VULNERABILITY: Arbitrary File Write! Path traversal allowed writing to absolute path {}",
                path
            );
        }
    }
}
